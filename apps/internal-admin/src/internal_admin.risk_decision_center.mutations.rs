use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::risk_decision_center_types::{RiskActionResult, action_result};
use crate::risk_decision_center_validation::validate_reason;

pub(crate) async fn approve_policy(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    decide_policy(db, access, workspace_id, policy_id, "approved", reason).await
}

pub(crate) async fn block_policy(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    decide_policy(db, access, workspace_id, policy_id, "blocked", reason).await
}

pub(crate) async fn resolve_risk_signal(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    signal_id: Uuid,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_risk_signals
         SET signal_value = signal_value || jsonb_build_object(
           'resolution_status', 'resolved',
           'resolution_reason', $1,
           'resolved_by', $2::text,
           'resolved_at', NOW()
         )
         WHERE id = $3 AND (tenant_id = $4 OR tenant_id IS NULL)
           AND COALESCE(signal_value->>'resolution_status', '') <> 'resolved'
         RETURNING id, tenant_id, signal_value",
    )
    .bind(&reason)
    .bind(access.actor_principal_id)
    .bind(signal_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "risk_signal_not_resolvable",
            "Risk signal is missing, belongs to another tenant, or is already resolved.",
        )
    })?;
    let metadata: Value = row.get("signal_value");
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RiskActionInput {
            action_kind: "resolve_risk_signal",
            policy_snapshot_id: None,
            risk_signal_id: Some(signal_id),
            previous_state: None,
            next_state: "resolved",
            reason,
            metadata,
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "risk.signal.resolved",
        "billing_risk_signal",
        signal_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "resolve_risk_signal",
        "resolved",
        "risk.signal.resolved",
    ))
}

async fn decide_policy(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    policy_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<RiskActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, policy_state AS previous_state
           FROM billing_access_policy_snapshots
           WHERE id = $3 AND tenant_id = $4
             AND (workspace_id = $5 OR workspace_id IS NULL)
             AND policy_state <> $1
         ),
         updated AS (
           UPDATE billing_access_policy_snapshots aps
           SET policy_state = $1, reason = $2
           FROM previous
           WHERE aps.id = previous.id
           RETURNING aps.policy_state AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(next_state)
    .bind(&reason)
    .bind(policy_id)
    .bind(access.tenant_id)
    .bind(workspace_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "risk_policy_not_decidable",
            "Risk policy is missing, belongs to another tenant, or is already in the requested state.",
        )
    })?;
    let action_kind = if next_state == "approved" {
        "approve_risk_policy"
    } else {
        "block_risk_policy"
    };
    let audit_action = if next_state == "approved" {
        "risk.policy.approved"
    } else {
        "risk.policy.blocked"
    };
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RiskActionInput {
            action_kind,
            policy_snapshot_id: Some(policy_id),
            risk_signal_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state,
            reason,
            metadata: json!({ "policy_snapshot_id": policy_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        "billing_access_policy_snapshot",
        policy_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        action_kind,
        next_state,
        audit_action,
    ))
}

struct RiskActionInput {
    action_kind: &'static str,
    policy_snapshot_id: Option<Uuid>,
    risk_signal_id: Option<Uuid>,
    previous_state: Option<String>,
    next_state: &'static str,
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
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    action_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, $6, jsonb_build_object('risk_action_id', $7), gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(action_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
