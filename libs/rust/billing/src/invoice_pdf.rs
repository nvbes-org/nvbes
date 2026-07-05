use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::invoices::{InvoiceDocument, InvoiceLine, InvoiceTotals, invoice_document_pdf_bytes};

pub struct InvoicePdfDownload {
    pub filename: String,
    pub body: Vec<u8>,
}

pub async fn fetch_invoice_pdf(
    db: &PgPool,
    workspace_id: Uuid,
    invoice_id: Uuid,
) -> Result<InvoicePdfDownload, sqlx::Error> {
    let invoice = fetch_invoice_document_record(db, workspace_id, invoice_id).await?;
    let lines = fetch_invoice_lines(db, invoice_id).await?;
    let document = InvoiceDocument {
        invoice_number: invoice
            .invoice_number
            .clone()
            .unwrap_or_else(|| invoice.invoice_id.to_string()),
        issue_date: invoice
            .issued_at
            .map(|value| value.date_naive().to_string())
            .unwrap_or_else(|| "not-issued".to_string()),
        seller_name: "nvbes".to_string(),
        customer_name: invoice.customer_name,
        customer_vat_id: invoice.customer_vat_id,
        totals: InvoiceTotals {
            subtotal_minor: invoice.subtotal_minor,
            tax_minor: invoice.tax_minor,
            total_minor: invoice.total_minor,
        },
        lines,
        currency: invoice.currency,
    };

    Ok(InvoicePdfDownload {
        filename: format!(
            "invoice-{}.pdf",
            safe_invoice_filename_part(&document.invoice_number)
        ),
        body: invoice_document_pdf_bytes(&document),
    })
}

async fn fetch_invoice_document_record(
    db: &PgPool,
    workspace_id: Uuid,
    invoice_id: Uuid,
) -> Result<InvoiceDocumentRecord, sqlx::Error> {
    sqlx::query_as::<_, InvoiceDocumentRecord>(
        r#"
        SELECT
          i.id AS invoice_id,
          i.invoice_number,
          i.subtotal_minor,
          i.tax_minor,
          i.total_minor,
          i.currency::text AS currency,
          i.issued_at,
          COALESCE(NULLIF(a.legal_name, ''), cp.billing_email, 'Customer') AS customer_name,
          tp.vat_id AS customer_vat_id
        FROM billing_invoices i
        JOIN billing_accounts a ON a.id = i.billing_account_id
        LEFT JOIN billing_customer_profiles cp ON cp.billing_account_id = a.id
        LEFT JOIN billing_tax_profiles tp ON tp.billing_account_id = a.id
        WHERE i.id = $1
          AND a.workspace_id = $2
          AND i.tenant_id = a.tenant_id
        "#,
    )
    .bind(invoice_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}

async fn fetch_invoice_lines(
    db: &PgPool,
    invoice_id: Uuid,
) -> Result<Vec<InvoiceLine>, sqlx::Error> {
    let records = sqlx::query_as::<_, InvoiceLineRecord>(
        r#"
        SELECT
          line_type,
          description,
          quantity::bigint AS quantity,
          unit_amount_minor,
          amount_minor,
          tax_minor
        FROM billing_invoice_lines
        WHERE invoice_id = $1
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(invoice_id)
    .fetch_all(db)
    .await?;

    Ok(records.into_iter().map(Into::into).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct InvoiceDocumentRecord {
    invoice_id: Uuid,
    invoice_number: Option<String>,
    subtotal_minor: i64,
    tax_minor: i64,
    total_minor: i64,
    currency: String,
    issued_at: Option<DateTime<Utc>>,
    customer_name: String,
    customer_vat_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct InvoiceLineRecord {
    line_type: String,
    description: String,
    quantity: i64,
    unit_amount_minor: i64,
    amount_minor: i64,
    tax_minor: i64,
}

impl From<InvoiceLineRecord> for InvoiceLine {
    fn from(record: InvoiceLineRecord) -> Self {
        Self {
            line_type: record.line_type,
            description: record.description,
            quantity: record.quantity,
            unit_amount_minor: record.unit_amount_minor,
            amount_minor: record.amount_minor,
            tax_minor: record.tax_minor,
        }
    }
}

fn safe_invoice_filename_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoices::calculate_invoice_totals;

    #[test]
    fn billing_invoice_pdf_filename_is_safe() {
        assert_eq!(
            safe_invoice_filename_part("NVBES/2026 000001"),
            "NVBES-2026-000001"
        );
    }

    #[test]
    fn invoice_totals_still_come_from_canonical_lines() {
        let totals = calculate_invoice_totals(&[InvoiceLine {
            line_type: "plan".to_string(),
            description: "Team".to_string(),
            quantity: 1,
            unit_amount_minor: 3_900,
            amount_minor: 3_900,
            tax_minor: 780,
        }]);

        assert_eq!(totals.total_minor, 4_680);
    }
}
