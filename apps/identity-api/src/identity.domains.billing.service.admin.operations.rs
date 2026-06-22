use super::mutations::{BillingAdminMutationResult, insert_admin_audit, insert_admin_ledger_pair};
use super::validate_admin_mutation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingGraceOverrideInput {
    pub tenant_id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub subscription_id: Option<uuid::Uuid>,
    pub grace_days: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingManualCompInput {
    pub tenant_id: uuid::Uuid,
    pub actor_principal_id: uuid::Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub direction: String,
    pub reason: String,
}

pub async fn override_grace_period(
    db: &sqlx::PgPool,
    input: BillingGraceOverrideInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(input.grace_days, &input.reason)?;
    let mut tx = db.begin().await?;
    let snapshot_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_access_policy_snapshots (
          tenant_id, workspace_id, policy_state, reason, effective_from, effective_to
        )
        VALUES ($1, $2, 'grace', $3, NOW(), NOW() + ($4 * INTERVAL '1 day'))
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(&input.reason)
    .bind(input.grace_days)
    .fetch_one(tx.as_mut())
    .await?;

    if let Some(subscription_id) = input.subscription_id {
        sqlx::query(
            r#"
            UPDATE billing_dunning_cases
            SET policy_state = 'grace', updated_at = NOW()
            WHERE tenant_id = $1
              AND subscription_id = $2
              AND status = 'open'
            "#,
        )
        .bind(input.tenant_id)
        .bind(subscription_id)
        .execute(tx.as_mut())
        .await?;
    }

    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.grace_period.overridden",
        "billing_access_policy_snapshot",
        snapshot_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: snapshot_id,
        ledger_entry_count: 0,
        audit_action: "billing.grace_period.overridden",
    })
}

pub async fn create_manual_compensation(
    db: &sqlx::PgPool,
    input: BillingManualCompInput,
) -> Result<BillingAdminMutationResult, crate::http::error::AppError> {
    validate_admin_mutation(input.amount_minor, &input.reason)?;
    let (debit_account, credit_account) = manual_comp_accounts(&input.direction)?;
    let mut tx = db.begin().await?;
    let adjustment_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO billing_adjustments (
          tenant_id, amount_minor, currency, direction, reason, created_by_principal_id
        )
        VALUES ($1, $2, $3, $4::billing_money_direction, $5, $6)
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.amount_minor)
    .bind(&input.currency)
    .bind(&input.direction)
    .bind(&input.reason)
    .bind(input.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;

    let ledger_entry_count = insert_admin_ledger_pair(
        &mut tx,
        input.tenant_id,
        "adjustment",
        adjustment_id,
        debit_account,
        credit_account,
        input.amount_minor,
        &input.currency,
        &input.reason,
    )
    .await?;
    insert_admin_audit(
        &mut tx,
        input.tenant_id,
        input.actor_principal_id,
        "billing.manual_comp.created",
        "billing_adjustment",
        adjustment_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(BillingAdminMutationResult {
        object_id: adjustment_id,
        ledger_entry_count,
        audit_action: "billing.manual_comp.created",
    })
}

fn manual_comp_accounts(
    direction: &str,
) -> Result<(&'static str, &'static str), crate::http::error::AppError> {
    match direction {
        "credit" => Ok(("adjustment", "accounts_receivable")),
        "debit" => Ok(("accounts_receivable", "adjustment")),
        _ => Err(crate::http::error::AppError::bad_request(
            "invalid_adjustment_direction",
            "Manual compensation direction must be credit or debit.",
        )),
    }
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
}
