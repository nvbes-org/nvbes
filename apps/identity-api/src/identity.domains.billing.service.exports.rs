use chrono::Utc;
use nvbes_billing::exports::{FinanceExportRow, FinanceExportType, export_filename, stable_csv};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct BillingFinanceExport {
    pub export_run_id: Uuid,
    pub filename: String,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub row_count: usize,
}

pub async fn build_finance_export(
    db: &PgPool,
    tenant_id: Uuid,
    export_type: FinanceExportType,
) -> Result<BillingFinanceExport, AppError> {
    let (headers, rows) = export_rows(db, tenant_id, &export_type).await?;
    let period = Utc::now().format("%Y-%m-%d").to_string();
    let filename = export_filename(export_type.clone(), &period);
    let csv = stable_csv(&headers, &rows);

    let export_run_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_export_runs (
          export_type, status, output_uri, summary
        )
        VALUES (
          $1, 'completed', 'inline-download',
          jsonb_build_object('row_count', $2, 'tenant_id', $3::text)
        )
        RETURNING id
        "#,
    )
    .bind(export_type_code(&export_type))
    .bind(i64::try_from(rows.len()).unwrap_or(i64::MAX))
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    Ok(BillingFinanceExport {
        export_run_id,
        filename,
        content_type: "text/csv; charset=utf-8",
        body: csv.into_bytes(),
        row_count: rows.len(),
    })
}

pub fn parse_finance_export_type(value: &str) -> Result<FinanceExportType, AppError> {
    match value {
        "invoices" => Ok(FinanceExportType::Invoices),
        "payments" => Ok(FinanceExportType::Payments),
        "tax" => Ok(FinanceExportType::Tax),
        "ledger" => Ok(FinanceExportType::Ledger),
        "customers" => Ok(FinanceExportType::Customers),
        "subscriptions" => Ok(FinanceExportType::Subscriptions),
        _ => Err(AppError::bad_request(
            "invalid_billing_export_type",
            "Billing export type must be invoices, payments, tax, ledger, customers, or subscriptions.",
        )),
    }
}

fn export_type_code(export_type: &FinanceExportType) -> &'static str {
    match export_type {
        FinanceExportType::Invoices => "invoices",
        FinanceExportType::Payments => "payments",
        FinanceExportType::Tax => "tax",
        FinanceExportType::Ledger => "ledger",
        FinanceExportType::Customers => "customers",
        FinanceExportType::Subscriptions => "subscriptions",
    }
}

async fn export_rows(
    db: &PgPool,
    tenant_id: Uuid,
    export_type: &FinanceExportType,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    match export_type {
        FinanceExportType::Invoices => export_invoices(db, tenant_id).await,
        FinanceExportType::Payments => export_payments(db, tenant_id).await,
        FinanceExportType::Tax => export_tax(db, tenant_id).await,
        FinanceExportType::Ledger => export_ledger(db, tenant_id).await,
        FinanceExportType::Customers => export_customers(db, tenant_id).await,
        FinanceExportType::Subscriptions => export_subscriptions(db, tenant_id).await,
    }
}

async fn export_invoices(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, COALESCE(invoice_number, ''), status::text, currency::text,
               subtotal_minor::text, tax_minor::text, total_minor::text,
               COALESCE(issued_at::text, ''), COALESCE(paid_at::text, '')
        FROM billing_invoices
        WHERE tenant_id = $1
        ORDER BY created_at DESC, id DESC
        LIMIT 5000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "invoice_id",
            "invoice_number",
            "status",
            "currency",
            "subtotal_minor",
            "tax_minor",
            "total_minor",
            "issued_at",
            "paid_at",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

async fn export_payments(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, COALESCE(invoice_id::text, ''), provider::text, status::text,
               currency::text, amount_minor::text, created_at::text
        FROM billing_payments
        WHERE tenant_id = $1
        ORDER BY created_at DESC, id DESC
        LIMIT 5000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "payment_id",
            "invoice_id",
            "provider",
            "status",
            "currency",
            "amount_minor",
            "created_at",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

async fn export_tax(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, COALESCE(invoice_number, ''), currency::text,
               tax_minor::text, COALESCE(issued_at::text, '')
        FROM billing_invoices
        WHERE tenant_id = $1
          AND tax_minor > 0
        ORDER BY issued_at DESC NULLS LAST, id DESC
        LIMIT 5000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "invoice_id",
            "invoice_number",
            "currency",
            "tax_minor",
            "issued_at",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

async fn export_ledger(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, entry_type::text, source_type, source_id::text,
               account_code, currency::text, amount_minor::text, occurred_at::text
        FROM billing_ledger_entries
        WHERE tenant_id = $1
        ORDER BY occurred_at DESC, id DESC
        LIMIT 10000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "ledger_entry_id",
            "entry_type",
            "source_type",
            "source_id",
            "account_code",
            "currency",
            "amount_minor",
            "occurred_at",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

async fn export_customers(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, workspace_id::text, COALESCE(legal_name, ''),
               COALESCE(billing_email, ''), currency::text, status
        FROM billing_accounts
        WHERE tenant_id = $1
        ORDER BY created_at DESC, id DESC
        LIMIT 5000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "billing_account_id",
            "workspace_id",
            "legal_name",
            "billing_email",
            "currency",
            "status",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

async fn export_subscriptions(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<(Vec<&'static str>, Vec<FinanceExportRow>), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id::text, COALESCE(workspace_id::text, ''), status, interval_unit,
               COALESCE(current_period_start::text, ''),
               COALESCE(current_period_end::text, '')
        FROM billing_subscriptions
        WHERE tenant_id = $1
        ORDER BY updated_at DESC, id DESC
        LIMIT 5000
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;
    Ok((
        vec![
            "subscription_id",
            "workspace_id",
            "status",
            "interval_unit",
            "current_period_start",
            "current_period_end",
        ],
        rows.into_iter().map(row_columns).collect(),
    ))
}

fn row_columns(row: sqlx::postgres::PgRow) -> FinanceExportRow {
    FinanceExportRow {
        columns: (0..row.len())
            .map(|index| row.try_get::<String, _>(index).unwrap_or_default())
            .collect(),
    }
}

#[cfg(test)]
#[path = "identity.domains.billing.service.exports.tests.rs"]
mod tests;
