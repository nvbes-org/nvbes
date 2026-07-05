use sqlx::Row;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{AdminBillingActionRequest, AdminBillingActionResult};
use crate::grpc::service_admin_billing::result;
use crate::grpc::service_admin_billing_ledger::{validate_admin_mutation, validate_provider_code};

pub async fn replay_provider_event(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.provider)?;
    let row = sqlx::query(
        "UPDATE billing_provider_events
         SET status = 'replayed', updated_at = NOW()
         WHERE tenant_id = $1 AND provider = $2::billing_provider
           AND provider_event_id = $3 AND status IN ('failed', 'rejected')
         RETURNING id, provider::text, provider_event_id, status::text",
    )
    .bind(tenant_id)
    .bind(&request.provider)
    .bind(&request.provider_event_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "provider_event_not_replayable: Provider event is missing for this tenant or not in a replayable state.",
        )
    })?;

    Ok(AdminBillingActionResult {
        object_id: row.get::<Uuid, _>(0).to_string(),
        ledger_entry_count: 0,
        audit_action: "billing.provider_event.replayed".to_string(),
        target_type: "billing_provider_event".to_string(),
        provider: row.get(1),
        provider_event_id: row.get(2),
        status: row.get(3),
    })
}

pub async fn create_provider_migration(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.from_provider)?;
    validate_provider_code(&request.to_provider)?;
    if request.from_provider == request.to_provider {
        return Err(Status::invalid_argument(
            "invalid_provider_migration: Provider migration requires distinct source and target providers.",
        ));
    }

    let migration_run_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_provider_migration_runs (
           tenant_id, from_provider, to_provider, status, summary
         ) VALUES (
           $1, $2::billing_provider, $3::billing_provider, 'planned',
           jsonb_build_object('reason', $4, 'created_by_principal_id', $5::text)
         ) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&request.from_provider)
    .bind(&request.to_provider)
    .bind(&request.reason)
    .bind(actor_principal_id)
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(result(
        migration_run_id,
        0,
        "billing.provider_migration.planned",
        "billing_provider_migration_run",
    ))
}

pub async fn override_grace_period(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    validate_admin_mutation(request.grace_days, &request.reason)?;
    let subscription_id = crate::grpc::service_status::optional_uuid(
        &request.subscription_id,
        "subscription_id",
    )?;
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let snapshot_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_access_policy_snapshots (
           tenant_id, workspace_id, policy_state, reason, effective_from, effective_to
         ) VALUES ($1, $2, 'grace', $3, NOW(), NOW() + ($4 * INTERVAL '1 day'))
         RETURNING id",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&request.reason)
    .bind(request.grace_days)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    if let Some(subscription_id) = subscription_id {
        sqlx::query(
            "UPDATE billing_dunning_cases
             SET policy_state = 'grace', updated_at = NOW()
             WHERE tenant_id = $1 AND subscription_id = $2 AND status = 'open'",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .execute(tx.as_mut())
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    }
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;

    Ok(result(
        snapshot_id,
        0,
        "billing.grace_period.overridden",
        "billing_access_policy_snapshot",
    ))
}

