use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::operations_center_action_log::{OperationsActionInput, insert_action, insert_audit};
use crate::operations_center_types::{OperationsActionResult, action_result};
use crate::operations_center_validation::validate_reason;

pub(crate) async fn replay_provider_event(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    event_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status::text AS previous_state
           FROM billing_provider_events
           WHERE id = $1 AND (tenant_id = $2 OR tenant_id IS NULL)
             AND status::text IN ('failed', 'rejected', 'processed')
         ),
         updated AS (
           UPDATE billing_provider_events bpe
           SET status = 'received', processed_at = NULL, updated_at = NOW()
           FROM previous
           WHERE bpe.id = previous.id
           RETURNING bpe.status::text AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(event_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "provider_event_not_replayable",
            "Provider event is missing, belongs to another tenant, or is not replayable.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "replay_provider_event",
            provider_event_id: Some(event_id),
            export_run_id: None,
            reconciliation_difference_id: None,
            incident_id: None,
            maintenance_window_id: None,
            job_run_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state: "received",
            reason,
            metadata: json!({ "provider_event_id": event_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.provider_event.replayed",
        "billing_provider_event",
        event_id,
        action_id,
        json!({
            "object_links": {
                "provider_event_id": event_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": "received",
                },
                {
                    "field": "processed_at",
                    "before": "recorded",
                    "after": null,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "replay_provider_event",
        "received",
        "operations.provider_event.replayed",
    ))
}

pub(crate) async fn replay_export_run(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    export_run_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    transition_export_run(db, access, workspace_id, export_run_id, reason).await
}

pub(crate) async fn resolve_reconciliation_difference(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    difference_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_reconciliation_differences
         SET resolved_at = NOW(),
           details = details || jsonb_build_object(
             'resolution_reason', $1,
             'resolved_by', $2::text
           )
         WHERE id = $3 AND (tenant_id = $4 OR tenant_id IS NULL) AND resolved_at IS NULL
         RETURNING id",
    )
    .bind(&reason)
    .bind(access.actor_principal_id)
    .bind(difference_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "reconciliation_difference_not_resolvable",
            "Reconciliation difference is missing, belongs to another tenant, or is already resolved.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "resolve_reconciliation_difference",
            provider_event_id: None,
            export_run_id: None,
            reconciliation_difference_id: Some(row.get("id")),
            incident_id: None,
            maintenance_window_id: None,
            job_run_id: None,
            previous_state: None,
            next_state: "resolved",
            reason,
            metadata: json!({ "reconciliation_difference_id": difference_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.reconciliation_difference.resolved",
        "billing_reconciliation_difference",
        difference_id,
        action_id,
        json!({
            "object_links": {
                "reconciliation_difference_id": difference_id,
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
                    "after": access.actor_principal_id,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "resolve_reconciliation_difference",
        "resolved",
        "operations.reconciliation_difference.resolved",
    ))
}

async fn transition_export_run(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    export_run_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM billing_export_runs
           WHERE id = $1 AND status IN ('failed', 'error', 'completed')
         ),
         updated AS (
           UPDATE billing_export_runs ber
           SET status = 'pending', updated_at = NOW()
           FROM previous
           WHERE ber.id = previous.id
           RETURNING ber.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(export_run_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "export_run_not_replayable",
            "Export run is missing or is not replayable.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "replay_export_run",
            provider_event_id: None,
            export_run_id: Some(export_run_id),
            reconciliation_difference_id: None,
            incident_id: None,
            maintenance_window_id: None,
            job_run_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state: "pending",
            reason,
            metadata: json!({ "export_run_id": export_run_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.export_run.replayed",
        "billing_export_run",
        export_run_id,
        action_id,
        json!({
            "object_links": {
                "export_run_id": export_run_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": "pending",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "replay_export_run",
        "pending",
        "operations.export_run.replayed",
    ))
}
