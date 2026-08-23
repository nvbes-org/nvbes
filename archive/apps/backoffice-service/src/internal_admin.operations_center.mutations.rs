use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc_operations::{
    BackofficeOperationsActionKind, BackofficeOperationsActionOutcome, run_admin_operations_action,
};
use crate::error::AppError;
use crate::operations_center_action_log::{OperationsActionInput, insert_action, insert_audit};
use crate::operations_center_types::{OperationsActionResult, action_result};
use crate::operations_center_validation::validate_reason;

pub(crate) async fn replay_provider_event(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    event_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_admin_operations_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeOperationsActionKind::ReplayProviderEvent,
        event_id,
        reason.clone(),
    )
    .await?;
    record_billing_action(
        db,
        access,
        workspace_id,
        reason,
        ActionTarget::ProviderEvent(event_id),
        outcome,
    )
    .await
}

pub(crate) async fn replay_export_run(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    export_run_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_admin_operations_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeOperationsActionKind::ReplayExportRun,
        export_run_id,
        reason.clone(),
    )
    .await?;
    record_billing_action(
        db,
        access,
        workspace_id,
        reason,
        ActionTarget::ExportRun(export_run_id),
        outcome,
    )
    .await
}

pub(crate) async fn resolve_reconciliation_difference(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    difference_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_admin_operations_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeOperationsActionKind::ResolveReconciliationDifference,
        difference_id,
        reason.clone(),
    )
    .await?;
    record_billing_action(
        db,
        access,
        workspace_id,
        reason,
        ActionTarget::ReconciliationDifference(difference_id),
        outcome,
    )
    .await
}

enum ActionTarget {
    ProviderEvent(Uuid),
    ExportRun(Uuid),
    ReconciliationDifference(Uuid),
}

async fn record_billing_action(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    reason: String,
    target: ActionTarget,
    outcome: BackofficeOperationsActionOutcome,
) -> Result<OperationsActionResult, AppError> {
    let action_kind = action_kind(&outcome.action_kind)?;
    let next_state = action_status(&outcome.status)?;
    let audit_action = audit_action(&outcome.audit_action)?;
    let target_type = target_type(&outcome.target_type)?;
    let metadata = action_metadata(workspace_id, &target, &outcome);
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind,
            provider_event_id: matches_target_id(&target, "provider_event"),
            export_run_id: matches_target_id(&target, "export_run"),
            reconciliation_difference_id: matches_target_id(&target, "reconciliation_difference"),
            incident_id: None,
            maintenance_window_id: None,
            job_run_id: None,
            previous_state: outcome.previous_state.clone(),
            next_state,
            reason,
            metadata: action_links(&target),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        target_type,
        outcome.object_id,
        action_id,
        metadata,
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

fn matches_target_id(target: &ActionTarget, field: &str) -> Option<Uuid> {
    match (target, field) {
        (ActionTarget::ProviderEvent(id), "provider_event") => Some(*id),
        (ActionTarget::ExportRun(id), "export_run") => Some(*id),
        (ActionTarget::ReconciliationDifference(id), "reconciliation_difference") => Some(*id),
        _ => None,
    }
}

fn action_links(target: &ActionTarget) -> Value {
    match target {
        ActionTarget::ProviderEvent(id) => json!({ "provider_event_id": id }),
        ActionTarget::ExportRun(id) => json!({ "export_run_id": id }),
        ActionTarget::ReconciliationDifference(id) => {
            json!({ "reconciliation_difference_id": id })
        }
    }
}

fn action_metadata(
    workspace_id: Uuid,
    target: &ActionTarget,
    outcome: &BackofficeOperationsActionOutcome,
) -> Value {
    match target {
        ActionTarget::ProviderEvent(id) => json!({
            "object_links": {
                "provider_event_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": outcome.previous_state,
                    "after": outcome.status,
                },
                {
                    "field": "processed_at",
                    "before": "recorded",
                    "after": null,
                }
            ],
        }),
        ActionTarget::ExportRun(id) => json!({
            "object_links": {
                "export_run_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": outcome.previous_state,
                    "after": outcome.status,
                }
            ],
        }),
        ActionTarget::ReconciliationDifference(id) => json!({
            "object_links": {
                "reconciliation_difference_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "resolved_at",
                    "before": null,
                    "after": "recorded",
                },
                {
                    "field": "details.resolution_reason",
                    "before": null,
                    "after": "recorded",
                },
                {
                    "field": "details.resolved_by",
                    "before": null,
                    "after": "recorded",
                }
            ],
        }),
    }
}

fn action_kind(value: &str) -> Result<&'static str, AppError> {
    match value {
        "replay_provider_event" => Ok("replay_provider_event"),
        "replay_export_run" => Ok("replay_export_run"),
        "resolve_reconciliation_difference" => Ok("resolve_reconciliation_difference"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_kind",
            format!("unexpected operations action kind: {value}"),
        )),
    }
}

fn action_status(value: &str) -> Result<&'static str, AppError> {
    match value {
        "received" => Ok("received"),
        "pending" => Ok("pending"),
        "resolved" => Ok("resolved"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_status",
            format!("unexpected operations action status: {value}"),
        )),
    }
}

fn audit_action(value: &str) -> Result<&'static str, AppError> {
    match value {
        "operations.provider_event.replayed" => Ok("operations.provider_event.replayed"),
        "operations.export_run.replayed" => Ok("operations.export_run.replayed"),
        "operations.reconciliation_difference.resolved" => {
            Ok("operations.reconciliation_difference.resolved")
        }
        _ => Err(AppError::internal(
            "billing_grpc_invalid_audit_action",
            format!("unexpected operations audit action: {value}"),
        )),
    }
}

fn target_type(value: &str) -> Result<&'static str, AppError> {
    match value {
        "billing_provider_event" => Ok("billing_provider_event"),
        "billing_export_run" => Ok("billing_export_run"),
        "billing_reconciliation_difference" => Ok("billing_reconciliation_difference"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_target_type",
            format!("unexpected operations target type: {value}"),
        )),
    }
}
