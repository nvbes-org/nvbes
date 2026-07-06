use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminBillingActionKind, AdminBillingActionRequest, AdminBillingActionResult,
};

pub async fn run_billing_admin_action(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    action_kind: AdminBillingActionKind,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    match action_kind {
        AdminBillingActionKind::CreateCreditNote => {
            validate_admin_mutation(request.amount_minor, &request.reason)?;
            create_credit_note(db, tenant_id, request).await
        }
        AdminBillingActionKind::CreateWriteOff => {
            validate_admin_mutation(request.amount_minor, &request.reason)?;
            create_write_off(db, tenant_id, actor_principal_id, request).await
        }
        AdminBillingActionKind::CreateRefundIntent => {
            validate_admin_mutation(request.amount_minor, &request.reason)?;
            create_refund_intent(db, tenant_id, request).await
        }
        AdminBillingActionKind::CreateManualCompensation => {
            validate_admin_mutation(request.amount_minor, &request.reason)?;
            create_manual_compensation(db, tenant_id, actor_principal_id, request).await
        }
        AdminBillingActionKind::ReplayProviderEvent => {
            crate::grpc::service_admin_billing_workflow::replay_provider_event(
                db, tenant_id, request,
            )
            .await
        }
        AdminBillingActionKind::CreateProviderMigration => {
            crate::grpc::service_admin_billing_workflow::create_provider_migration(
                db,
                tenant_id,
                actor_principal_id,
                request,
            )
            .await
        }
        AdminBillingActionKind::OverrideGracePeriod => {
            let workspace_id = parse_required_uuid(&request.workspace_id, "workspace_id")?;
            crate::grpc::service_admin_billing_workflow::override_grace_period(
                db,
                tenant_id,
                workspace_id,
                request,
            )
            .await
        }
        AdminBillingActionKind::Unspecified => Err(Status::invalid_argument(
            "billing admin action kind is required",
        )),
    }
}

async fn create_credit_note(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    let invoice_id = parse_required_uuid(&request.invoice_id, "invoice_id")?;
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let credit_note_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_credit_notes (tenant_id, invoice_id, amount_minor, currency, reason)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(tenant_id)
    .bind(invoice_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        tenant_id,
        "credit_note",
        credit_note_id,
        "credit_note",
        "accounts_receivable",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    Ok(result(
        credit_note_id,
        ledger_entry_count,
        "billing.credit_note.created",
        "billing_credit_note",
    ))
}

async fn create_write_off(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    let invoice_id = parse_required_uuid(&request.invoice_id, "invoice_id")?;
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let write_off_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_write_offs (
           tenant_id, invoice_id, amount_minor, currency, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(invoice_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .bind(actor_principal_id)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        tenant_id,
        "write_off",
        write_off_id,
        "write_off",
        "accounts_receivable",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    Ok(result(
        write_off_id,
        ledger_entry_count,
        "billing.write_off.created",
        "billing_write_off",
    ))
}

async fn create_refund_intent(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    validate_provider_code(&request.provider)?;
    let payment_id = parse_required_uuid(&request.payment_id, "payment_id")?;
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let refund_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_refunds (
           tenant_id, payment_id, provider, amount_minor, currency, reason, status
         ) VALUES ($1, $2, $3::billing_provider, $4, $5, $6, 'pending') RETURNING id",
    )
    .bind(tenant_id)
    .bind(payment_id)
    .bind(&request.provider)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.reason)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        tenant_id,
        "refund",
        refund_id,
        "refund",
        "cash",
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    Ok(result(
        refund_id,
        ledger_entry_count,
        "billing.refund_intent.created",
        "billing_refund",
    ))
}

async fn create_manual_compensation(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    request: AdminBillingActionRequest,
) -> Result<AdminBillingActionResult, Status> {
    let (debit_account, credit_account) = manual_comp_accounts(&request.direction)?;
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let adjustment_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_adjustments (
           tenant_id, amount_minor, currency, direction, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4::billing_money_direction, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(request.amount_minor)
    .bind(&request.currency)
    .bind(&request.direction)
    .bind(&request.reason)
    .bind(actor_principal_id)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        tenant_id,
        "adjustment",
        adjustment_id,
        debit_account,
        credit_account,
        request.amount_minor,
        &request.currency,
        &request.reason,
    )
    .await?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    Ok(result(
        adjustment_id,
        ledger_entry_count,
        "billing.manual_comp.created",
        "billing_adjustment",
    ))
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
) -> Result<u64, Status> {
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
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    Ok(result.rows_affected())
}

pub(crate) fn result(
    object_id: Uuid,
    ledger_entry_count: u64,
    audit_action: &'static str,
    target_type: &'static str,
) -> AdminBillingActionResult {
    AdminBillingActionResult {
        object_id: object_id.to_string(),
        ledger_entry_count,
        audit_action: audit_action.to_string(),
        target_type: target_type.to_string(),
        provider: String::new(),
        provider_event_id: String::new(),
        status: "completed".to_string(),
    }
}

pub(crate) fn validate_admin_mutation(amount_minor: i64, reason: &str) -> Result<(), Status> {
    if amount_minor <= 0 {
        return Err(Status::invalid_argument(
            "invalid_amount: Billing admin amount must be greater than zero.",
        ));
    }
    if reason.trim().len() < 12 {
        return Err(Status::invalid_argument(
            "audit_reason_required: Billing admin actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

pub(crate) fn validate_provider_code(provider: &str) -> Result<(), Status> {
    if nvbes_billing::provider_code(provider).is_some() {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_billing_provider: Billing provider is not supported.",
    ))
}

fn manual_comp_accounts(direction: &str) -> Result<(&'static str, &'static str), Status> {
    match direction {
        "credit" => Ok(("adjustment", "accounts_receivable")),
        "debit" => Ok(("accounts_receivable", "adjustment")),
        _ => Err(Status::invalid_argument(
            "invalid_adjustment_direction: Manual compensation direction must be credit or debit.",
        )),
    }
}

fn parse_required_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    if value.trim().is_empty() {
        return Err(Status::invalid_argument(format!("{field} is required")));
    }
    crate::grpc::service_status::parse_uuid(value, field)
}

#[cfg(test)]
mod tests {
    use super::{manual_comp_accounts, validate_admin_mutation, validate_provider_code};

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

    #[test]
    fn admin_mutation_requires_positive_amount() {
        assert!(validate_admin_mutation(1, "ticket BILL-123").is_ok());
        assert!(validate_admin_mutation(0, "ticket BILL-123").is_err());
    }
}
