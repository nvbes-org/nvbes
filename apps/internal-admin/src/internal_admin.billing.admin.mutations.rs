use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::{
    BackofficeAccess, CreditNoteRequest, GraceOverrideRequest, ManualCompRequest, MutationResult,
    ProviderMigrationRequest, ProviderReplayRequest, ProviderReplayResult, RefundIntentRequest,
};
use crate::billing_admin_validation::{validate_admin_mutation, validate_provider_code};
use crate::error::AppError;

fn mutation_result(
    object_id: Uuid,
    ledger_entry_count: u64,
    audit_action: &'static str,
) -> MutationResult {
    MutationResult {
        object_id,
        ledger_entry_count,
        audit_action,
    }
}

pub(crate) async fn create_credit_note(
    db: &PgPool,
    access: BackofficeAccess,
    request: CreditNoteRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    let mut tx = db.begin().await?;
    let credit_note_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_credit_notes (tenant_id, invoice_id, amount_minor, currency, reason)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(request.invoice_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        access.tenant_id,
        "credit_note",
        credit_note_id,
        "credit_note",
        "accounts_receivable",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        access,
        "billing.credit_note.created",
        "billing_credit_note",
        credit_note_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        credit_note_id,
        ledger_entry_count,
        "billing.credit_note.created",
    ))
}

pub(crate) async fn create_write_off(
    db: &PgPool,
    access: BackofficeAccess,
    request: CreditNoteRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    let mut tx = db.begin().await?;
    let write_off_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_write_offs (
           tenant_id, invoice_id, amount_minor, currency, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(request.invoice_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        access.tenant_id,
        "write_off",
        write_off_id,
        "write_off",
        "accounts_receivable",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        access,
        "billing.write_off.created",
        "billing_write_off",
        write_off_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        write_off_id,
        ledger_entry_count,
        "billing.write_off.created",
    ))
}

pub(crate) async fn create_refund_intent(
    db: &PgPool,
    access: BackofficeAccess,
    request: RefundIntentRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    validate_provider_code(&request.provider)?;
    let mut tx = db.begin().await?;
    let refund_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_refunds (
           tenant_id, payment_id, provider, amount_minor, currency, reason, status
         ) VALUES ($1, $2, $3::billing_provider, $4, $5, $6, 'pending') RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(request.payment_id)
    .bind(&request.provider)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        access.tenant_id,
        "refund",
        refund_id,
        "refund",
        "cash",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        access,
        "billing.refund_intent.created",
        "billing_refund",
        refund_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        refund_id,
        ledger_entry_count,
        "billing.refund_intent.created",
    ))
}

pub(crate) async fn replay_provider_event(
    db: &PgPool,
    access: BackofficeAccess,
    request: ProviderReplayRequest,
) -> Result<ProviderReplayResult, AppError> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.provider)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_provider_events
         SET status = 'replayed', updated_at = NOW()
         WHERE tenant_id = $1 AND provider = $2::billing_provider
           AND provider_event_id = $3 AND status IN ('failed', 'rejected')
         RETURNING id, provider::text, provider_event_id, status::text",
    )
    .bind(access.tenant_id)
    .bind(&request.provider)
    .bind(&request.provider_event_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "provider_event_not_replayable",
            "Provider event is missing for this tenant or not in a replayable state.",
        )
    })?;
    let object_id: Uuid = row.get(0);
    insert_admin_audit(
        &mut tx,
        access,
        "billing.provider_event.replayed",
        "billing_provider_event",
        object_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(ProviderReplayResult {
        object_id,
        provider: row.get(1),
        provider_event_id: row.get(2),
        status: row.get(3),
        audit_action: "billing.provider_event.replayed",
    })
}

pub(crate) async fn create_provider_migration(
    db: &PgPool,
    access: BackofficeAccess,
    request: ProviderMigrationRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.from_provider)?;
    validate_provider_code(&request.to_provider)?;
    if request.from_provider == request.to_provider {
        return Err(AppError::bad_request(
            "invalid_provider_migration",
            "Provider migration requires distinct source and target providers.",
        ));
    }

    let mut tx = db.begin().await?;
    let migration_run_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_provider_migration_runs (
           tenant_id, from_provider, to_provider, status, summary
         ) VALUES (
           $1, $2::billing_provider, $3::billing_provider, 'planned',
           jsonb_build_object('reason', $4, 'created_by_principal_id', $5::text)
         ) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(&request.from_provider)
    .bind(&request.to_provider)
    .bind(&request.reason)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    insert_admin_audit(
        &mut tx,
        access,
        "billing.provider_migration.planned",
        "billing_provider_migration_run",
        migration_run_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        migration_run_id,
        0,
        "billing.provider_migration.planned",
    ))
}

pub(crate) async fn override_grace_period(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: GraceOverrideRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.grace_days, &request.reason)?;
    let mut tx = db.begin().await?;
    let snapshot_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_access_policy_snapshots (
           tenant_id, workspace_id, policy_state, reason, effective_from, effective_to
         ) VALUES ($1, $2, 'grace', $3, NOW(), NOW() + ($4 * INTERVAL '1 day'))
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(&request.reason)
    .bind(request.grace_days)
    .fetch_one(tx.as_mut())
    .await?;
    if let Some(subscription_id) = request.subscription_id {
        sqlx::query(
            "UPDATE billing_dunning_cases
             SET policy_state = 'grace', updated_at = NOW()
             WHERE tenant_id = $1 AND subscription_id = $2 AND status = 'open'",
        )
        .bind(access.tenant_id)
        .bind(subscription_id)
        .execute(tx.as_mut())
        .await?;
    }
    insert_admin_audit(
        &mut tx,
        access,
        "billing.grace_period.overridden",
        "billing_access_policy_snapshot",
        snapshot_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        snapshot_id,
        0,
        "billing.grace_period.overridden",
    ))
}

pub(crate) async fn create_manual_compensation(
    db: &PgPool,
    access: BackofficeAccess,
    request: ManualCompRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    let (debit_account, credit_account) = manual_comp_accounts(&request.direction)?;
    let mut tx = db.begin().await?;
    let adjustment_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_adjustments (
           tenant_id, amount_minor, currency, direction, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4::billing_money_direction, $5, $6) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.direction)
    .bind(&request.reason)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        access.tenant_id,
        "adjustment",
        adjustment_id,
        debit_account,
        credit_account,
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        access,
        "billing.manual_comp.created",
        "billing_adjustment",
        adjustment_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(mutation_result(
        adjustment_id,
        ledger_entry_count,
        "billing.manual_comp.created",
    ))
}

fn manual_comp_accounts(direction: &str) -> Result<(&'static str, &'static str), AppError> {
    match direction {
        "credit" => Ok(("adjustment", "accounts_receivable")),
        "debit" => Ok(("accounts_receivable", "adjustment")),
        _ => Err(AppError::bad_request(
            "invalid_adjustment_direction",
            "Manual compensation direction must be credit or debit.",
        )),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Ledger writes keep each accounting dimension explicit."
)]
async fn insert_admin_ledger_pair(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    source_type: &str,
    source_id: Uuid,
    debit_account: &str,
    credit_account: &str,
    amount_minor: i64,
    currency: &str,
    reason: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "INSERT INTO billing_ledger_entries (
           tenant_id, entry_type, source_type, source_id, account_code, currency, amount_minor, metadata
         ) VALUES
           ($1, $2::billing_ledger_entry_type, $3, $4, $5, $6, $7, jsonb_build_object('reason', $8)),
           ($1, $2::billing_ledger_entry_type, $3, $4, $9, $6, -$7, jsonb_build_object('reason', $8))",
    )
    .bind(tenant_id)
    .bind(source_type)
    .bind(source_type)
    .bind(source_id)
    .bind(debit_account)
    .bind(currency)
    .bind(amount_minor)
    .bind(reason)
    .bind(credit_account)
    .execute(tx.as_mut())
    .await?;
    Ok(result.rows_affected())
}

async fn insert_admin_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_type: &str,
    target_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object(
             'reason', $6,
             'object_links', jsonb_build_object(
               'target_id', $5::text,
               'target_type', $4
             ),
             'target_links', jsonb_build_object(
               'target_id', $5::text,
               'target_type', $4
             ),
             'changes', jsonb_build_array(jsonb_build_object(
               'field', 'billing_admin.action',
               'before', null,
               'after', 'recorded'
             ))
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_comp_accounts_are_directional() {
        assert_eq!(
            manual_comp_accounts("credit").unwrap(),
            ("adjustment", "accounts_receivable")
        );
        assert!(manual_comp_accounts("invalid").is_err());
    }

    #[test]
    fn provider_code_validation_accepts_supported_psps_only() {
        assert!(validate_provider_code("stripe").is_ok());
        assert!(validate_provider_code("mollie").is_ok());
        assert!(validate_provider_code("paypal").is_err());
    }
}
