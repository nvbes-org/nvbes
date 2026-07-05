use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminOperationsActionKind, AdminOperationsActionResult,
};

pub async fn run_operations_action(
    db: &sqlx::PgPool,
    kind: AdminOperationsActionKind,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_id: Uuid,
    reason: String,
) -> Result<AdminOperationsActionResult, Status> {
    validate_reason(&reason)?;
    match kind {
        AdminOperationsActionKind::ReplayProviderEvent => {
            replay_provider_event(db, tenant_id, target_id).await
        }
        AdminOperationsActionKind::ReplayExportRun => replay_export_run(db, target_id).await,
        AdminOperationsActionKind::ResolveReconciliationDifference => {
            resolve_reconciliation_difference(db, tenant_id, actor_principal_id, target_id, reason)
                .await
        }
        AdminOperationsActionKind::Unspecified => Err(Status::invalid_argument(
            "admin operations action kind is required",
        )),
    }
}

async fn replay_provider_event(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    event_id: Uuid,
) -> Result<AdminOperationsActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
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
        JOIN updated ON TRUE
        "#,
    )
    .bind(event_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "provider_event_not_replayable: Provider event is missing, belongs to another tenant, or is not replayable.",
        )
    })?;

    Ok(AdminOperationsActionResult {
        object_id: event_id.to_string(),
        action_kind: "replay_provider_event".to_string(),
        status: row.1,
        audit_action: "operations.provider_event.replayed".to_string(),
        target_type: "billing_provider_event".to_string(),
        previous_state: row.0,
    })
}

async fn replay_export_run(
    db: &sqlx::PgPool,
    export_run_id: Uuid,
) -> Result<AdminOperationsActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
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
        JOIN updated ON TRUE
        "#,
    )
    .bind(export_run_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "export_run_not_replayable: Export run is missing or is not replayable.",
        )
    })?;

    Ok(AdminOperationsActionResult {
        object_id: export_run_id.to_string(),
        action_kind: "replay_export_run".to_string(),
        status: row.1,
        audit_action: "operations.export_run.replayed".to_string(),
        target_type: "billing_export_run".to_string(),
        previous_state: row.0,
    })
}

async fn resolve_reconciliation_difference(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    difference_id: Uuid,
    reason: String,
) -> Result<AdminOperationsActionResult, Status> {
    let row = sqlx::query_as::<_, (Uuid,)>(
        r#"
        UPDATE billing_reconciliation_differences
        SET resolved_at = NOW(),
          details = details || jsonb_build_object(
            'resolution_reason', $1,
            'resolved_by', $2::text
          )
        WHERE id = $3 AND (tenant_id = $4 OR tenant_id IS NULL) AND resolved_at IS NULL
        RETURNING id
        "#,
    )
    .bind(&reason)
    .bind(actor_principal_id)
    .bind(difference_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "reconciliation_difference_not_resolvable: Reconciliation difference is missing, belongs to another tenant, or is already resolved.",
        )
    })?;

    Ok(AdminOperationsActionResult {
        object_id: row.0.to_string(),
        action_kind: "resolve_reconciliation_difference".to_string(),
        status: "resolved".to_string(),
        audit_action: "operations.reconciliation_difference.resolved".to_string(),
        target_type: "billing_reconciliation_difference".to_string(),
        previous_state: String::new(),
    })
}

fn validate_reason(value: &str) -> Result<(), Status> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_operations_reason: Operations action reason must contain between 8 and 500 characters.",
    ))
}

