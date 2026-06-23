use super::validate_admin_mutation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingAdminMutationInput {
    pub tenant_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub invoice_id: uuid::Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingRefundIntentInput {
    pub tenant_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub payment_id: uuid::Uuid,
    pub provider: String,
    pub amount_minor: i64,
    pub currency: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BillingAdminMutationResult {
    pub object_id: uuid::Uuid,
    pub ledger_entry_count: u64,
    pub audit_action: &'static str,
}

pub async fn create_credit_note(
    db: &sqlx::PgPool,
    input: BillingAdminMutationInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(input.amount_minor, &input.reason)?;
    let mut tx = db.begin().await?;
    let credit_note_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_credit_notes (tenant_id, invoice_id, amount_minor, currency, reason)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.invoice_id)
    .bind(input.amount_minor)
    .bind(&input.currency)
    .bind(&input.reason)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        input.tenant_id,
        "credit_note",
        credit_note_id,
        "credit_note",
        "accounts_receivable",
        input.amount_minor,
        &input.currency,
        &input.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.credit_note.created",
        "billing_credit_note",
        credit_note_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: credit_note_id,
        ledger_entry_count,
        audit_action: "billing.credit_note.created",
    })
}

pub async fn create_write_off(
    db: &sqlx::PgPool,
    input: BillingAdminMutationInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(input.amount_minor, &input.reason)?;
    let mut tx = db.begin().await?;
    let write_off_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_write_offs (
          tenant_id, invoice_id, amount_minor, currency, reason, created_by_principal_id
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.invoice_id)
    .bind(input.amount_minor)
    .bind(&input.currency)
    .bind(&input.reason)
    .bind(input.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        input.tenant_id,
        "write_off",
        write_off_id,
        "write_off",
        "accounts_receivable",
        input.amount_minor,
        &input.currency,
        &input.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.write_off.created",
        "billing_write_off",
        write_off_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: write_off_id,
        ledger_entry_count,
        audit_action: "billing.write_off.created",
    })
}

pub async fn create_refund_intent(
    db: &sqlx::PgPool,
    input: BillingRefundIntentInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(input.amount_minor, &input.reason)?;
    let mut tx = db.begin().await?;
    let refund_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_refunds (
          tenant_id, payment_id, provider, amount_minor, currency, reason, status
        )
        VALUES ($1, $2, $3::billing_provider, $4, $5, $6, 'pending')
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.payment_id)
    .bind(&input.provider)
    .bind(input.amount_minor)
    .bind(&input.currency)
    .bind(&input.reason)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        input.tenant_id,
        "refund",
        refund_id,
        "refund",
        "cash",
        input.amount_minor,
        &input.currency,
        &input.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.refund_intent.created",
        "billing_refund",
        refund_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: refund_id,
        ledger_entry_count,
        audit_action: "billing.refund_intent.created",
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "Ledger writes keep each accounting dimension explicit."
)]
pub(super) async fn insert_admin_ledger_pair(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: uuid::Uuid,
    source_type: &str,
    source_id: uuid::Uuid,
    debit_account: &str,
    credit_account: &str,
    amount_minor: i64,
    currency: &str,
    reason: &str,
) -> Result<u64, crate::http::error::AppError> {
    let result = sqlx::query(
        r#"
        INSERT INTO billing_ledger_entries (
          tenant_id, entry_type, source_type, source_id, account_code, currency, amount_minor, metadata
        )
        VALUES
          ($1, $2::billing_ledger_entry_type, $3, $4, $5, $6, $7, jsonb_build_object('reason', $8)),
          ($1, $2::billing_ledger_entry_type, $3, $4, $9, $6, -$7, jsonb_build_object('reason', $8))
        "#,
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

pub(super) async fn insert_admin_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: uuid::Uuid,
    actor_principal_id: uuid::Uuid,
    action: &'static str,
    target_type: &str,
    target_id: uuid::Uuid,
    reason: &str,
) -> Result<(), crate::http::error::AppError> {
    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: None,
            actor_principal_id: Some(actor_principal_id),
            action,
            target_type,
            target_id: Some(target_id),
            ip: None,
            user_agent: None,
            metadata: serde_json::json!({ "reason": reason }),
        },
    )
    .await
}
