use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1::{
    AdminUsageActionKind, AdminUsageActionRequest, AdminUsageActionResult,
};

#[derive(Debug, Serialize)]
pub(crate) struct UsageActionResult {
    pub(crate) object_id: Uuid,
    pub(crate) action_kind: String,
    pub(crate) status: String,
    pub(crate) audit_action: String,
}

pub(crate) struct UsageCorrectionInput {
    pub(crate) usage_event_id: Option<Uuid>,
    pub(crate) meter_code: String,
    pub(crate) quantity_delta: i64,
    pub(crate) reason: String,
}

pub(crate) async fn correct_usage(
    billing_grpc_endpoint: &str,
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

    let result = crate::billing_grpc::run_usage_grpc_action(
        billing_grpc_endpoint,
        access,
        workspace_id,
        AdminUsageActionRequest {
            context: None,
            workspace_id: String::new(),
            action_kind: AdminUsageActionKind::CorrectUsage as i32,
            usage_event_id: input
                .usage_event_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            meter_code: input.meter_code,
            quantity_delta: input.quantity_delta,
            target_id: String::new(),
            reason: input.reason,
        },
    )
    .await?;
    let object_id = parse_uuid(&result.object_id)?;
    let metadata = parse_metadata_json(&result)?;
    let mut tx = db.begin().await?;
    insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: result.action_kind.clone(),
            meter_code: result.meter_code.clone(),
            target_id: Some(object_id),
            quantity_delta: Some(result.quantity_delta),
            status: result.status.clone(),
            reason: result.description.clone(),
            metadata: metadata.clone(),
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        &result.audit_action,
        &result.target_type,
        object_id,
        &result.description,
        json!({
            "object_links": {
                "usage_correction_id": object_id,
                "usage_event_id": metadata.get("usage_event_id").cloned().unwrap_or(Value::Null),
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "usage_correction",
                    "before": null,
                    "after": "created",
                },
                {
                    "field": "quantity_delta",
                    "before": 0,
                    "after": result.quantity_delta,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(result_from_grpc(object_id, result))
}

pub(crate) async fn freeze_meter(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    meter_code: String,
    reason: String,
) -> Result<UsageActionResult, AppError> {
    validate_meter_code(&meter_code)?;
    validate_reason(&reason)?;
    let result = crate::billing_grpc::run_usage_grpc_action(
        billing_grpc_endpoint,
        access,
        workspace_id,
        AdminUsageActionRequest {
            context: None,
            workspace_id: String::new(),
            action_kind: AdminUsageActionKind::FreezeMeter as i32,
            usage_event_id: String::new(),
            meter_code,
            quantity_delta: 0,
            target_id: String::new(),
            reason,
        },
    )
    .await?;
    let meter_id = parse_uuid(&result.object_id)?;
    let metadata = parse_metadata_json(&result)?;
    let mut tx = db.begin().await?;
    let action_id = insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: result.action_kind.clone(),
            meter_code: result.meter_code.clone(),
            target_id: Some(meter_id),
            quantity_delta: None,
            status: result.status.clone(),
            reason: result.description.clone(),
            metadata,
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        &result.audit_action,
        "internal_admin_usage_action",
        action_id,
        &result.description,
        json!({
            "object_links": {
                "meter_id": meter_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "meter.status",
                    "before": "active",
                    "after": "frozen",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(result_from_grpc(action_id, result))
}

pub(crate) async fn replay_rollup(
    billing_grpc_endpoint: &str,
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rollup_id: Uuid,
    reason: String,
) -> Result<UsageActionResult, AppError> {
    validate_reason(&reason)?;
    let result = crate::billing_grpc::run_usage_grpc_action(
        billing_grpc_endpoint,
        access,
        workspace_id,
        AdminUsageActionRequest {
            context: None,
            workspace_id: String::new(),
            action_kind: AdminUsageActionKind::ReplayRollup as i32,
            usage_event_id: String::new(),
            meter_code: String::new(),
            quantity_delta: 0,
            target_id: rollup_id.to_string(),
            reason,
        },
    )
    .await?;
    let metadata = parse_metadata_json(&result)?;
    let mut tx = db.begin().await?;
    let action_id = insert_usage_action(
        &mut tx,
        access,
        workspace_id,
        UsageActionInput {
            action_kind: result.action_kind.clone(),
            meter_code: result.meter_code.clone(),
            target_id: Some(rollup_id),
            quantity_delta: None,
            status: result.status.clone(),
            reason: result.description.clone(),
            metadata,
        },
    )
    .await?;
    insert_usage_audit(
        &mut tx,
        access,
        &result.audit_action,
        &result.target_type,
        rollup_id,
        &result.description,
        json!({
            "object_links": {
                "rollup_id": rollup_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "updated_at",
                    "before": "recorded",
                    "after": "refreshed",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(result_from_grpc(action_id, result))
}

struct UsageActionInput {
    action_kind: String,
    meter_code: String,
    target_id: Option<Uuid>,
    quantity_delta: Option<i64>,
    status: String,
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
    action: &str,
    target_type: &str,
    target_id: Uuid,
    description: &str,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('description', $6) || $7::jsonb,
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(description)
    .bind(metadata)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn result_from_grpc(object_id: Uuid, result: AdminUsageActionResult) -> UsageActionResult {
    UsageActionResult {
        object_id,
        action_kind: result.action_kind,
        status: result.status,
        audit_action: result.audit_action,
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", "invalid uuid"))
}

fn parse_metadata_json(result: &AdminUsageActionResult) -> Result<Value, AppError> {
    serde_json::from_str(&result.metadata_json)
        .map_err(|error| AppError::internal("billing_grpc_decode", error.to_string()))
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
