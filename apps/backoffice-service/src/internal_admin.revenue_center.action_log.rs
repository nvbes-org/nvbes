use serde_json::Value;
use sqlx::Transaction;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;

pub(crate) struct RevenueActionInput {
    pub(crate) action_kind: &'static str,
    pub(crate) dunning_case_id: Option<Uuid>,
    pub(crate) invoice_id: Option<Uuid>,
    pub(crate) dispute_id: Option<Uuid>,
    pub(crate) previous_state: Option<String>,
    pub(crate) next_state: &'static str,
    pub(crate) reason: String,
    pub(crate) metadata: Value,
}

pub(crate) async fn insert_action(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RevenueActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_revenue_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, dunning_case_id,
           invoice_id, dispute_id, previous_state, next_state, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.dunning_case_id)
    .bind(input.invoice_id)
    .bind(input.dispute_id)
    .bind(input.previous_state)
    .bind(input.next_state)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

pub(crate) async fn insert_audit(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    action_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, $6,
           jsonb_build_object('revenue_action_id', $7) || $8::jsonb,
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(action_id)
    .bind(metadata)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
