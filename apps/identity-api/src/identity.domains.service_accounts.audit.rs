use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domains::{authz::WorkspaceAccess, service_accounts::policy::required_tenant_id},
    http::error::AppError,
};

pub async fn record_audit_event(
    tx: &mut Transaction<'_, Postgres>,
    access: &WorkspaceAccess,
    action: &str,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), AppError> {
    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: required_tenant_id(access)?,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
            action,
            target_type: "service_account",
            target_id,
            ip,
            user_agent,
            metadata,
        },
    )
    .await
}
