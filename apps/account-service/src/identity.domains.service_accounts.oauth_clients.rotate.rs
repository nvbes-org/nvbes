use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        oauth,
        service_accounts::{
            audit::record_audit_event,
            core::get_service_account,
            policy::{enforce_target_role_management, ensure_attached_client},
            types::RotateOAuthClientSecretResult,
        },
    },
    http::error::AppError,
};

pub async fn rotate_oauth_client_secret(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<RotateOAuthClientSecretResult, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;
    ensure_attached_client(&service_account, client_id)?;

    let client_secret = format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let client_secret_hash = oauth::hash_client_secret(&client_secret)?;
    let rotated_at = Utc::now();
    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET client_secret_hash = $2
        WHERE client_id = $1
        "#,
    )
    .bind(client_id)
    .bind(&client_secret_hash)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET last_rotated_at = $2,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(service_account_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.secret_rotated",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;

    Ok(RotateOAuthClientSecretResult {
        client_id: client_id.to_string(),
        client_secret,
        rotated_at,
    })
}
