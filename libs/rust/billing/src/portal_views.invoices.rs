use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalInvoiceProviderView {
    pub provider: String,
    pub status: String,
    pub invoice_number: Option<String>,
    pub pdf_available: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalInvoiceView {
    pub invoice_id: Uuid,
    pub invoice_number: Option<String>,
    pub canonical_pdf_url: String,
    pub status: String,
    pub total_minor: i64,
    pub currency: String,
    pub issued_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub providers: Vec<BillingPortalInvoiceProviderView>,
}

pub async fn fetch_portal_invoices(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<BillingPortalInvoiceView>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
          i.id AS invoice_id,
          i.invoice_number,
          i.status::text AS status,
          i.total_minor,
          i.currency::text AS currency,
          i.issued_at,
          i.due_at,
          i.paid_at,
          pi.provider::text AS provider,
          pi.provider_invoice_number,
          pi.provider_pdf_url,
          pi.status AS provider_status
        FROM billing_invoices i
        JOIN billing_accounts a ON a.id = i.billing_account_id
        LEFT JOIN billing_provider_invoices pi ON pi.invoice_id = i.id
        WHERE a.workspace_id = $1
        ORDER BY COALESCE(i.issued_at, i.created_at) DESC, i.id, pi.provider::text
        LIMIT 48
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    let mut invoices = Vec::<BillingPortalInvoiceView>::new();
    for row in rows {
        let invoice_id: Uuid = row.get("invoice_id");
        let invoice_index = invoices
            .iter()
            .position(|invoice| invoice.invoice_id == invoice_id);
        let index = match invoice_index {
            Some(index) => index,
            None => {
                invoices.push(BillingPortalInvoiceView {
                    invoice_id,
                    invoice_number: row.get("invoice_number"),
                    canonical_pdf_url: canonical_invoice_pdf_url(workspace_id, invoice_id),
                    status: row.get("status"),
                    total_minor: row.get("total_minor"),
                    currency: row.get("currency"),
                    issued_at: row.get("issued_at"),
                    due_at: row.get("due_at"),
                    paid_at: row.get("paid_at"),
                    providers: Vec::new(),
                });
                invoices.len() - 1
            }
        };

        let provider: Option<String> = row.get("provider");
        if let Some(provider) = provider {
            let pdf_url: Option<String> = row.get("provider_pdf_url");
            invoices[index]
                .providers
                .push(BillingPortalInvoiceProviderView {
                    provider,
                    status: row.get("provider_status"),
                    invoice_number: row.get("provider_invoice_number"),
                    pdf_available: pdf_url.is_some(),
                });
        }
    }

    invoices.truncate(24);
    Ok(invoices)
}

pub fn canonical_invoice_pdf_url(workspace_id: Uuid, invoice_id: Uuid) -> String {
    format!("/workspaces/{workspace_id}/billing/portal/invoices/{invoice_id}/pdf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_invoice_pdf_url_is_local_to_nvbes() {
        let workspace_id = Uuid::nil();
        let invoice_id = Uuid::nil();

        assert_eq!(
            canonical_invoice_pdf_url(workspace_id, invoice_id),
            "/workspaces/00000000-0000-0000-0000-000000000000/billing/portal/invoices/00000000-0000-0000-0000-000000000000/pdf"
        );
    }
}
