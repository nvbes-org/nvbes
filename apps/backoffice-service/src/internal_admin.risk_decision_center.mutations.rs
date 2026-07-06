use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1::{AdminRiskActionKind, AdminRiskActionResult};
use crate::risk_decision_center_types::{RiskActionResult, action_result};
use crate::risk_decision_center_validation::validate_reason;

pub(crate) async fn approve_policy(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    decide_policy(
        billing_grpc_endpoint,
        db,
        access,
        workspace_id,
        policy_id,
        AdminRiskActionKind::ApprovePolicy,
        reason,
    )
    .await
}

pub(crate) async fn block_policy(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    decide_policy(
        billing_grpc_endpoint,
        db,
        access,
        workspace_id,
        policy_id,
        AdminRiskActionKind::BlockPolicy,
        reason,
    )
    .await
}

pub(crate) async fn resolve_risk_signal(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    signal_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    validate_reason(&reason)?;
    let result = crate::billing_grpc::run_risk_grpc_action(
        billing_grpc_endpoint,
        access,
        workspace_id,
        AdminRiskActionKind::ResolveRiskSignal,
        signal_id,
        reason,
    )
    .await?;
    let metadata = parse_metadata_json(&result)?;
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RiskActionInput {
            action_kind: result.action_kind.clone(),
            policy_snapshot_id: None,
            risk_signal_id: Some(signal_id),
            previous_state: None,
            next_state: result.status.clone(),
            reason: metadata_reason(&metadata),
            metadata,
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        &result.audit_action,
        &result.target_type,
        signal_id,
        action_id,
        json!({
            "object_links": {
                "risk_signal_id": signal_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "resolution_status",
                    "before": "unresolved",
                    "after": "resolved",
                },
                {
                    "field": "resolution_reason",
                    "before": null,
                    "after": "recorded",
                },
                {
                    "field": "resolved_by",
                    "before": null,
                    "after": access.actor_principal_id,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        result.action_kind,
        result.status,
        result.audit_action,
    ))
}

async fn decide_policy(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    action_kind: AdminRiskActionKind,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    validate_reason(&reason)?;
    let result = crate::billing_grpc::run_risk_grpc_action(
        billing_grpc_endpoint,
        access,
        workspace_id,
        action_kind,
        policy_id,
        reason.clone(),
    )
    .await?;
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RiskActionInput {
            action_kind: result.action_kind.clone(),
            policy_snapshot_id: Some(policy_id),
            risk_signal_id: None,
            previous_state: empty_to_none(result.previous_state.clone()),
            next_state: result.status.clone(),
            reason,
            metadata: parse_metadata_json(&result)?,
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        &result.audit_action,
        &result.target_type,
        policy_id,
        action_id,
        json!({
            "object_links": {
                "policy_snapshot_id": policy_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "policy_state",
                    "before": empty_to_none(result.previous_state.clone()),
                    "after": result.status.clone(),
                },
                {
                    "field": "reason",
                    "before": null,
                    "after": "recorded",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        result.action_kind,
        result.status,
        result.audit_action,
    ))
}

struct RiskActionInput {
    action_kind: String,
    policy_snapshot_id: Option<Uuid>,
    risk_signal_id: Option<Uuid>,
    previous_state: Option<String>,
    next_state: String,
    reason: String,
    metadata: Value,
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RiskActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_risk_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, policy_snapshot_id,
           risk_signal_id, previous_state, next_state, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.policy_snapshot_id)
    .bind(input.risk_signal_id)
    .bind(input.previous_state)
    .bind(input.next_state)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

async fn insert_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    action: &str,
    target_type: &str,
    target_id: Uuid,
    action_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, $6,
           jsonb_build_object('risk_action_id', $7) || $8::jsonb,
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

fn parse_metadata_json(result: &AdminRiskActionResult) -> Result<Value, AppError> {
    serde_json::from_str(&result.metadata_json)
        .map_err(|error| AppError::internal("billing_grpc_decode", error.to_string()))
}

fn metadata_reason(metadata: &Value) -> String {
    metadata
        .get("resolution_reason")
        .and_then(Value::as_str)
        .unwrap_or("risk signal resolved")
        .to_string()
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
