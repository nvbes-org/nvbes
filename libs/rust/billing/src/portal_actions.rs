use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{BillingAuditEventInput, insert_billing_audit_event_tx};
use crate::portal_sessions::{PortalSessionError, create_provider_portal_session};
use crate::subscription_status_requires_lock;
use crate::types::{CreatePortalInput, PortalSessionResponse};

#[derive(Debug, Clone)]
pub struct CreateBillingPortalSessionInput {
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub portal: CreatePortalInput,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Error)]
pub enum BillingPortalActionError {
    #[error("workspace_not_found")]
    WorkspaceNotFound,
    #[error("billing_locked")]
    BillingLocked,
    #[error(transparent)]
    Portal(#[from] PortalSessionError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub async fn create_billing_portal_session(
    db: &PgPool,
    config: &AppConfig,
    input: CreateBillingPortalSessionInput,
) -> Result<PortalSessionResponse, BillingPortalActionError> {
    let mut tx = db.begin().await?;
    let record = crate::db::fetch_billing_state_tx(&mut tx, input.workspace_id)
        .await?
        .ok_or(BillingPortalActionError::WorkspaceNotFound)?;
    if subscription_status_requires_lock(&record.subscription_status) {
        return Err(BillingPortalActionError::BillingLocked);
    }

    let response = create_provider_portal_session(config, &record, input.portal).await?;
    insert_billing_audit_event_tx(
        &mut tx,
        BillingAuditEventInput {
            workspace_id: input.workspace_id,
            actor_principal_id: Some(input.actor_principal_id),
            action: "billing.portal_opened",
            target_type: "workspace",
            target_id: Some(input.workspace_id),
            ip: input.ip.as_deref(),
            user_agent: input.user_agent.as_deref(),
            metadata: serde_json::json!({
                "provider": response.provider,
            }),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(response)
}
