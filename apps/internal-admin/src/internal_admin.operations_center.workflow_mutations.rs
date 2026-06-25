use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::operations_center_action_log::{OperationsActionInput, insert_action, insert_audit};
use crate::operations_center_types::{OperationsActionResult, action_result};
use crate::operations_center_validation::validate_reason;

pub(crate) async fn update_incident_state(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    incident_id: Uuid,
    status: String,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let next_status = validate_incident_status(&status)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM internal_admin_incidents
           WHERE id = $1 AND tenant_id = $2 AND workspace_id = $3
         ),
         updated AS (
           UPDATE internal_admin_incidents incident
           SET status = $4,
             resolved_at = CASE WHEN $4 = 'resolved' THEN NOW() ELSE resolved_at END,
             updated_at = NOW()
           FROM previous
           WHERE incident.id = previous.id
           RETURNING incident.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(incident_id)
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(next_status)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "incident_not_mutable",
            "Incident is missing or belongs to another workspace.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "update_incident_state",
            provider_event_id: None,
            export_run_id: None,
            reconciliation_difference_id: None,
            incident_id: Some(incident_id),
            maintenance_window_id: None,
            job_run_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state: next_status,
            reason,
            metadata: json!({ "incident_id": incident_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.incident.state_updated",
        "internal_admin_incident",
        incident_id,
        action_id,
        json!({
            "object_links": {"incident_id": incident_id, "workspace_id": workspace_id},
            "changes": [{"field": "status", "before": row.get::<String, _>("previous_state"), "after": next_status}]
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "update_incident_state",
        next_status,
        "operations.incident.state_updated",
    ))
}

pub(crate) async fn schedule_maintenance_window(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    title: String,
    scheduled_start_at: DateTime<Utc>,
    scheduled_end_at: DateTime<Utc>,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    validate_maintenance_window(&title, scheduled_start_at, scheduled_end_at)?;
    let mut tx = db.begin().await?;
    let window_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_maintenance_windows (
           tenant_id, workspace_id, title, scheduled_start_at, scheduled_end_at, reason,
           created_by_principal_id
         ) VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(title.trim())
    .bind(scheduled_start_at)
    .bind(scheduled_end_at)
    .bind(&reason)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "schedule_maintenance_window",
            provider_event_id: None,
            export_run_id: None,
            reconciliation_difference_id: None,
            incident_id: None,
            maintenance_window_id: Some(window_id),
            job_run_id: None,
            previous_state: None,
            next_state: "scheduled",
            reason,
            metadata: json!({ "maintenance_window_id": window_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.maintenance_window.scheduled",
        "internal_admin_maintenance_window",
        window_id,
        action_id,
        json!({
            "object_links": {"maintenance_window_id": window_id, "workspace_id": workspace_id},
            "changes": [
                {"field": "status", "before": null, "after": "scheduled"},
                {"field": "scheduled_start_at", "before": null, "after": scheduled_start_at},
                {"field": "scheduled_end_at", "before": null, "after": scheduled_end_at}
            ]
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "schedule_maintenance_window",
        "scheduled",
        "operations.maintenance_window.scheduled",
    ))
}

pub(crate) async fn replay_job_run(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    job_run_id: Uuid,
    reason: String,
) -> Result<OperationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM internal_admin_job_runs
           WHERE id = $1 AND tenant_id = $2 AND workspace_id = $3
             AND status IN ('failed', 'completed', 'cancelled')
         ),
         updated AS (
           UPDATE internal_admin_job_runs job
           SET status = 'queued', last_error = NULL, started_at = NULL, completed_at = NULL, updated_at = NOW()
           FROM previous
           WHERE job.id = previous.id
           RETURNING job.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(job_run_id)
    .bind(access.tenant_id)
    .bind(workspace_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "job_run_not_replayable",
            "Job run is missing, belongs to another workspace, or is not replayable.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        OperationsActionInput {
            action_kind: "replay_job_run",
            provider_event_id: None,
            export_run_id: None,
            reconciliation_difference_id: None,
            incident_id: None,
            maintenance_window_id: None,
            job_run_id: Some(job_run_id),
            previous_state: Some(row.get("previous_state")),
            next_state: "queued",
            reason,
            metadata: json!({ "job_run_id": job_run_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "operations.job_run.replayed",
        "internal_admin_job_run",
        job_run_id,
        action_id,
        json!({
            "object_links": {"job_run_id": job_run_id, "workspace_id": workspace_id},
            "changes": [{"field": "status", "before": row.get::<String, _>("previous_state"), "after": "queued"}]
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "replay_job_run",
        "queued",
        "operations.job_run.replayed",
    ))
}

fn validate_incident_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim() {
        "open" => Ok("open"),
        "mitigating" => Ok("mitigating"),
        "resolved" => Ok("resolved"),
        _ => Err(AppError::bad_request(
            "invalid_incident_status",
            "Incident status must be open, mitigating, or resolved.",
        )),
    }
}

fn validate_maintenance_window(
    title: &str,
    scheduled_start_at: DateTime<Utc>,
    scheduled_end_at: DateTime<Utc>,
) -> Result<(), AppError> {
    if title.trim().len() < 4 {
        return Err(AppError::bad_request(
            "invalid_maintenance_title",
            "Maintenance window title must contain at least 4 characters.",
        ));
    }
    if scheduled_end_at <= scheduled_start_at {
        return Err(AppError::bad_request(
            "invalid_maintenance_window",
            "Maintenance window end must be after its start.",
        ));
    }
    Ok(())
}
