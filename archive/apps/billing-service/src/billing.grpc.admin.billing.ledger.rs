use tonic::Status;
use uuid::Uuid;

#[expect(
    clippy::too_many_arguments,
    reason = "Ledger writes keep each accounting dimension explicit."
)]
pub async fn insert_admin_ledger_pair(
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

pub fn validate_admin_mutation(amount_minor: i64, reason: &str) -> Result<(), Status> {
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

pub fn validate_provider_code(provider: &str) -> Result<(), Status> {
    if nvbes_billing::provider_code(provider).is_some() {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_billing_provider: Billing provider is not supported.",
    ))
}

pub fn manual_comp_accounts(direction: &str) -> Result<(&'static str, &'static str), Status> {
    match direction {
        "credit" => Ok(("adjustment", "accounts_receivable")),
        "debit" => Ok(("accounts_receivable", "adjustment")),
        _ => Err(Status::invalid_argument(
            "invalid_adjustment_direction: Manual compensation direction must be credit or debit.",
        )),
    }
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
