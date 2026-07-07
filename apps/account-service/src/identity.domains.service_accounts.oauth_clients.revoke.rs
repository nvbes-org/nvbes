use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            audit::record_audit_event,
            core::get_service_account,
            policy::{enforce_target_role_management, ensure_attached_client},
            types::ServiceAccountView,
        },
    },
    http::error::AppError,
};

pub async fn revoke_oauth_client(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;
    ensure_attached_client(&service_account, client_id)?;

    let mut tx = db.begin().await?;
    let client_uuid = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET revoked_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(client_uuid)
    .execute(&mut *tx)
    .await?;

    let _ = nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client_uuid)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;
    let _ = nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, client_id)
        .await
        .map_err(|err| {
            AppError::internal(
                "pushed_authorization_request_revoke_failed",
                err.to_string(),
            )
        })?;

    crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
        redis, client_id,
    )
    .await?;
    crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, client_id).await?;

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET client_id = NULL,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(service_account_id)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.revoked",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.detached",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, service_account_id).await
}
