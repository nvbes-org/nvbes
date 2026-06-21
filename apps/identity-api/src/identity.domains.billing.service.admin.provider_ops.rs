use super::mutations::{BillingAdminMutationResult, insert_admin_audit};
use super::{validate_admin_mutation, validate_provider_code};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingProviderEventReplayInput {
    pub tenant_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub provider: String,
    pub provider_event_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingProviderMigrationInput {
    pub tenant_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub from_provider: String,
    pub to_provider: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BillingProviderEventReplayResult {
    pub object_id: uuid::Uuid,
    pub provider: String,
    pub provider_event_id: String,
    pub status: String,
    pub audit_action: &'static str,
}

pub async fn replay_provider_event(
    db: &sqlx::PgPool,
    input: BillingProviderEventReplayInput,
) -> Result<BillingProviderEventReplayResult, crate::http::error::AppError> {
    validate_admin_mutation(1, &input.reason)?;
    validate_provider_code(&input.provider)?;
    let mut tx = db.begin().await?;
    let record = sqlx::query_as::<_, crate::domains::billing::types::ProviderEventRecord>(
        r#"
        UPDATE billing_provider_events
        SET status = 'replayed',
            updated_at = NOW()
        WHERE tenant_id = $1
          AND provider = $2::billing_provider
          AND provider_event_id = $3
          AND status IN ('failed', 'rejected')
        RETURNING
          id,
          tenant_id,
          provider::text AS provider,
          provider_event_id,
          event_type,
          status::text AS status,
          signature_valid,
          payload_summary
        "#,
    )
    .bind(input.tenant_id)
    .bind(&input.provider)
    .bind(&input.provider_event_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        crate::http::error::AppError::conflict(
            "provider_event_not_replayable",
            "Provider event is missing for this tenant or not in a replayable state.",
        )
    })?;

    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.provider_event.replayed",
        "billing_provider_event",
        record.id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingProviderEventReplayResult {
        object_id: record.id,
        provider: record.provider,
        provider_event_id: record.provider_event_id,
        status: record.status,
        audit_action: "billing.provider_event.replayed",
    })
}

pub async fn create_provider_migration_run(
    db: &sqlx::PgPool,
    input: BillingProviderMigrationInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(1, &input.reason)?;
    validate_provider_code(&input.from_provider)?;
    validate_provider_code(&input.to_provider)?;
    if input.from_provider == input.to_provider {
        return Err(crate::http::error::AppError::bad_request(
            "invalid_provider_migration",
            "Provider migration requires distinct source and target providers.",
        ));
    }

    let mut tx = db.begin().await?;
    let migration_run_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_provider_migration_runs (
          tenant_id, from_provider, to_provider, status, summary
        )
        VALUES (
          $1, $2::billing_provider, $3::billing_provider, 'planned',
          jsonb_build_object('reason', $4, 'created_by_principal_id', $5::text)
        )
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(&input.from_provider)
    .bind(&input.to_provider)
    .bind(&input.reason)
    .bind(input.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;

    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.provider_migration.planned",
        "billing_provider_migration_run",
        migration_run_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: migration_run_id,
        ledger_entry_count: 0,
        audit_action: "billing.provider_migration.planned",
    })
}
