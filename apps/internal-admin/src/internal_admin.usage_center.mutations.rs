use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct UsageActionResult {
    pub(crate) object_id: Uuid,
    pub(crate) action_kind: &'static str,
    pub(crate) status: &'static str,
    pub(crate) audit_action: &'static str,
}

pub(crate) struct UsageCorrectionInput {
    pub(crate) usage_event_id: Option<Uuid>,
    pub(crate) meter_code: String,
    pub(crate) quantity_delta: i64,
    pub(crate) reason: String,
}

pub(crate) async fn correct_usage(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: UsageCorrectionInput,
) -> Result<UsageActionResult, AppError> {
    validate_meter_code(&input.meter_code)?;
    validate_reason(&input.reason)?;
    if input.quantity_delta == 0 {
        return Err(AppError::bad_request(
            "invalid_usage_correction",
            "Usage correction quantity delta cannot be zero.",
        ));
    }

    let mut tx = db.begin().await?;
    let correction_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_usage_corrections (
           tenant_id, usage_event_id, meter_code, quantity_delta, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(input.usage_event_id)
    .bind(&input.meter_code)
    .bind(input.quantity_delta)
    .bind(&input.reason)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: "correct_usage",
            meter_code: input.meter_code,
            target_id: Some(correction_id),
            quantity_delta: Some(input.quantity_delta),
            status: "applied",
            reason: input.reason,
            metadata: json!({ "usage_event_id": input.usage_event_id }),
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        "usage.correction.created",
        "billing_usage_correction",
        correction_id,
        "Usage correction applied.",
    )
    .await?;
    tx.commit().await?;
    Ok(result(
        correction_id,
        "correct_usage",
        "applied",
        "usage.correction.created",
    ))
}

pub(crate) async fn freeze_meter(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    meter_code: String,
    reason: String,
) -> Result<UsageActionResult, AppError> {
    validate_meter_code(&meter_code)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let meter_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE billing_meter_definitions
         SET status = 'frozen', updated_at = NOW()
         WHERE code = $1 AND status <> 'frozen'
         RETURNING id",
    )
    .bind(&meter_code)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "meter_not_freezable",
            "Meter is missing or already frozen.",
        )
    })?;
    let action_id = insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: "freeze_meter",
            meter_code,
            target_id: Some(meter_id),
            quantity_delta: None,
            status: "applied",
            reason,
            metadata: json!({ "meter_id": meter_id }),
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        "usage.meter.frozen",
        "internal_admin_usage_action",
        action_id,
        "Meter frozen by back-office.",
    )
    .await?;
    tx.commit().await?;
    Ok(result(
        action_id,
        "freeze_meter",
        "applied",
        "usage.meter.frozen",
    ))
}

pub(crate) async fn replay_rollup(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rollup_id: Uuid,
    reason: String,
) -> Result<UsageActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_usage_rollups
         SET updated_at = NOW()
         WHERE id = $1 AND tenant_id = $2
         RETURNING id, meter_code",
    )
    .bind(rollup_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| AppError::not_found("usage_rollup_not_found", "Usage rollup not found."))?;
    let meter_code: String = row.get("meter_code");
    let action_id = insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: "replay_rollup",
            meter_code,
            target_id: Some(rollup_id),
            quantity_delta: None,
            status: "replayed",
            reason,
            metadata: json!({ "rollup_id": rollup_id }),
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        "usage.rollup.replayed",
        "billing_usage_rollup",
        rollup_id,
        "Usage rollup replay requested.",
    )
    .await?;
    tx.commit().await?;
    Ok(result(
        action_id,
        "replay_rollup",
        "replayed",
        "usage.rollup.replayed",
    ))
}

struct UsageActionInput {
    action_kind: &'static str,
    meter_code: String,
    target_id: Option<Uuid>,
    quantity_delta: Option<i64>,
    status: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_usage_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: UsageActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_usage_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, meter_code,
           target_id, quantity_delta, status, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.meter_code)
    .bind(input.target_id)
    .bind(input.quantity_delta)
    .bind(input.status)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

async fn insert_usage_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    description: &'static str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, jsonb_build_object('description', $6), gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(description)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn result(
    object_id: Uuid,
    action_kind: &'static str,
    status: &'static str,
    audit_action: &'static str,
) -> UsageActionResult {
    UsageActionResult {
        object_id,
        action_kind,
        status,
        audit_action,
    }
}

pub(crate) fn validate_meter_code(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (2..=96).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_usage_meter_code",
        "Usage meter code must contain between 2 and 96 characters.",
    ))
}

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_usage_reason",
        "Usage action reason must contain between 8 and 500 characters.",
    ))
}
