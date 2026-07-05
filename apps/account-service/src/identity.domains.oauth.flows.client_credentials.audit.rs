use sqlx::postgres::PgPool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub(super) struct MachineTokenAuditInput<'a> {
    pub client_uuid: Uuid,
    pub client_id: &'a str,
    pub service_account_principal_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub jti: &'a str,
    pub scope: &'a str,
    pub audience: &'a str,
}

pub(super) async fn record_machine_token_issued(
    db: &PgPool,
    input: MachineTokenAuditInput<'_>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    mark_client_used(&mut tx, input.client_uuid).await?;
    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: input.tenant_id,
            workspace_id: Some(input.workspace_id),
            actor_principal_id: Some(input.service_account_principal_id),
            action: "oauth.machine_token.issued",
            target_type: "oauth_client",
            target_id: Some(input.client_uuid),
            ip: None,
            user_agent: None,
            metadata: machine_token_audit_metadata(
                input.client_id,
                input.jti,
                input.scope,
                input.audience,
            ),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn mark_client_used(
    tx: &mut Transaction<'_, Postgres>,
    client_uuid: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET last_used_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(client_uuid)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) fn machine_token_audit_metadata(
    client_id: &str,
    jti: &str,
    scope: &str,
    audience: &str,
) -> serde_json::Value {
    serde_json::json!({
        "grant_type": "client_credentials",
        "client_id": client_id,
        "jti": jti,
        "scope": scope,
        "audience": audience,
    })
}
