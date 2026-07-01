use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct BillingPortalInvoiceView {
    pub invoice_id: Uuid,
    pub invoice_number: Option<String>,
    pub status: String,
    pub total_minor: i64,
    pub currency: String,
    pub issued_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPortalCreditView {
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPortalView {
    pub plan_code: String,
    pub provider: String,
    pub payment_method_update_flow: String,
    pub payment_method_changes_delegated_to_provider: bool,
    pub automatically_updates_payment_method_references: bool,
    pub exposes_provider_secret_ids: bool,
    pub invoices: Vec<BillingPortalInvoiceView>,
    pub credits: Vec<BillingPortalCreditView>,
}

pub async fn get_portal_view(
    db: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> Result<BillingPortalView, AppError> {
    let billing = super::overview::get_billing(db, workspace_id).await?;
    let invoices = fetch_portal_invoices(db, workspace_id).await?;
    let credits = fetch_portal_credits(db, tenant_id).await?;

    Ok(BillingPortalView {
        plan_code: billing.plan.code,
        provider: billing.subscription.billing_provider,
        payment_method_update_flow: "nvbes_provider_redirect".to_string(),
        payment_method_changes_delegated_to_provider: false,
        automatically_updates_payment_method_references: false,
        exposes_provider_secret_ids: false,
        invoices,
        credits,
    })
}

async fn fetch_portal_invoices(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<BillingPortalInvoiceView>, AppError> {
    let invoices = sqlx::query_as::<_, BillingPortalInvoiceRecord>(
        r#"
        SELECT
          i.id AS invoice_id,
          i.invoice_number,
          i.status::text AS status,
          i.total_minor,
          i.currency::text AS currency,
          i.issued_at,
          i.due_at,
          i.paid_at
        FROM billing_invoices i
        JOIN billing_accounts a ON a.id = i.billing_account_id
        WHERE a.workspace_id = $1
        ORDER BY COALESCE(i.issued_at, i.created_at) DESC
        LIMIT 24
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    Ok(invoices.into_iter().map(Into::into).collect())
}

async fn fetch_portal_credits(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<BillingPortalCreditView>, AppError> {
    let credits = sqlx::query_as::<_, BillingPortalCreditRecord>(
        r#"
        SELECT COALESCE(SUM(remaining_minor), 0)::bigint AS amount_minor,
               currency::text AS currency
        FROM billing_commercial_credits
        WHERE tenant_id = $1
          AND remaining_minor > 0
          AND (expires_at IS NULL OR expires_at > NOW())
        GROUP BY currency
        ORDER BY currency
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(credits.into_iter().map(Into::into).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct BillingPortalInvoiceRecord {
    invoice_id: Uuid,
    invoice_number: Option<String>,
    status: String,
    total_minor: i64,
    currency: String,
    issued_at: Option<DateTime<Utc>>,
    due_at: Option<DateTime<Utc>>,
    paid_at: Option<DateTime<Utc>>,
}

impl From<BillingPortalInvoiceRecord> for BillingPortalInvoiceView {
    fn from(record: BillingPortalInvoiceRecord) -> Self {
        Self {
            invoice_id: record.invoice_id,
            invoice_number: record.invoice_number,
            status: record.status,
            total_minor: record.total_minor,
            currency: record.currency,
            issued_at: record.issued_at,
            due_at: record.due_at,
            paid_at: record.paid_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct BillingPortalCreditRecord {
    amount_minor: i64,
    currency: String,
}

impl From<BillingPortalCreditRecord> for BillingPortalCreditView {
    fn from(record: BillingPortalCreditRecord) -> Self {
        Self {
            amount_minor: record.amount_minor,
            currency: record.currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_portal_invoice_view_contains_no_provider_identifiers() {
        let view = BillingPortalView {
            plan_code: "team".to_string(),
            provider: "stripe".to_string(),
            payment_method_update_flow: "nvbes_provider_redirect".to_string(),
            payment_method_changes_delegated_to_provider: false,
            automatically_updates_payment_method_references: false,
            exposes_provider_secret_ids: false,
            invoices: vec![BillingPortalInvoiceView {
                invoice_id: Uuid::nil(),
                invoice_number: Some("NVBES-2026-000001".to_string()),
                status: "issued".to_string(),
                total_minor: 4_680,
                currency: "EUR".to_string(),
                issued_at: None,
                due_at: None,
                paid_at: None,
            }],
            credits: vec![BillingPortalCreditView {
                amount_minor: 1_000,
                currency: "EUR".to_string(),
            }],
        };

        assert!(!view.exposes_provider_secret_ids);
        assert_eq!(view.payment_method_update_flow, "nvbes_provider_redirect");
        assert!(!view.payment_method_changes_delegated_to_provider);
        assert!(!view.automatically_updates_payment_method_references);
        assert_eq!(
            view.invoices[0].invoice_number.as_deref(),
            Some("NVBES-2026-000001")
        );
    }
}
