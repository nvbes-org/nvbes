use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            audit::record_audit_event,
            core::get_service_account,
            policy::{enforce_target_role_management, required_tenant_id},
            types::ServiceAccountView,
        },
    },
    http::error::AppError,
};

pub async fn attach_oauth_client(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;

    let mut tx = db.begin().await?;
    let client = sqlx::query(
        r#"
        SELECT
          id,
          tenant_id,
          owner_scope_type::text AS owner_scope_type,
          owner_scope_id
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
          AND client_type = 'service'
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    let tenant_id: Option<Uuid> = client.get("tenant_id");
    if tenant_id != Some(required_tenant_id(access)?) {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "The OAuth client does not belong to this workspace tenant.",
        ));
    }
    let owner_scope_type: String = client.get("owner_scope_type");
    let owner_scope_id: Uuid = client.get("owner_scope_id");
    if owner_scope_type != "workspace" || owner_scope_id != access.workspace_id {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "The OAuth client does not belong to this workspace.",
        ));
    }

    let attached_elsewhere = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM service_accounts
          WHERE client_id = $1
            AND principal_id <> $2
        )
        "#,
    )
    .bind(client_id)
    .bind(service_account_id)
    .fetch_one(&mut *tx)
    .await?;

    if attached_elsewhere {
        return Err(AppError::conflict(
            "service_account_client_attached",
            "This OAuth client is already attached to another service account.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET client_id = $2,
            updated_at = NOW()
        WHERE principal_id = $1
          AND workspace_id = $3
        "#,
    )
    .bind(service_account_id)
    .bind(client_id)
    .bind(access.workspace_id)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.attached",
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
