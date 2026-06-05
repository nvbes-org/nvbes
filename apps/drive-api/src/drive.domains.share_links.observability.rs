use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub struct AuditEventInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'static str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}

pub async fn log_share_link_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: AuditEventInput<'_>,
) -> Result<(), AppError> {
    crate::domains::files::queries::insert_audit_event_tx(
        tx,
        crate::domains::files::queries::AuditEventInput {
            workspace_id: input.workspace_id,
            actor_user_id: input.actor_user_id,
            actor_principal_id: input.actor_principal_id,
            action: input.action,
            target_type: "share_link",
            target_id: input.target_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: input.metadata,
        },
    )
    .await
}
