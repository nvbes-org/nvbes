use chrono::Utc;
use nvbes_billing::exports::{FinanceExportRow, FinanceExportType, export_filename, stable_csv};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppError;

pub(crate) struct FinanceExport {
    pub(crate) export_run_id: Uuid,
    pub(crate) filename: String,
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
    pub(crate) row_count: usize,
}

pub(crate) async fn build_finance_export(
    db: &PgPool,
    tenant_id: Uuid,
    export_type: FinanceExportType,
) -> Result<FinanceExport, AppError> {
    let (headers, rows) = export_rows(db, tenant_id, &export_type).await?;
    let period = Utc::now().format("%Y-%m-%d").to_string();
    let filename = export_filename(export_type.clone(), &period);
    let csv = stable_csv(&headers, &rows);
    let export_run_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_export_runs (export_type, status, output_uri, summary)
         VALUES ($1, 'completed', 'inline-download', jsonb_build_object('row_count', $2, 'tenant_id', $3::text))
         RETURNING id",
    )
    .bind(export_type_code(&export_type))
    .bind(i64::try_from(rows.len()).unwrap_or(i64::MAX))
    .bind(tenant_id)
    .fetch_one(db)
    .await?;
    Ok(FinanceExport {
        export_run_id,
        filename,
        content_type: "text/csv; charset=utf-8",
        body: csv.into_bytes(),
        row_count: rows.len(),
    })
}

pub(crate) fn parse_finance_export_type(value: &str) -> Result<FinanceExportType, AppError> {
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
    let (table, columns, order, headers) = match export_type {
        FinanceExportType::Invoices => (
            "billing_invoices",
            "id::text, COALESCE(invoice_number, ''), status::text, currency::text, subtotal_minor::text, tax_minor::text, total_minor::text",
            "created_at DESC, id DESC",
            vec![
                "invoice_id",
                "invoice_number",
                "status",
                "currency",
                "subtotal_minor",
                "tax_minor",
                "total_minor",
            ],
        ),
        FinanceExportType::Payments => (
            "billing_payments",
            "id::text, COALESCE(invoice_id::text, ''), provider::text, status::text, currency::text, amount_minor::text",
            "created_at DESC, id DESC",
            vec![
                "payment_id",
                "invoice_id",
                "provider",
                "status",
                "currency",
                "amount_minor",
            ],
        ),
        FinanceExportType::Tax => (
            "billing_invoices",
            "id::text, COALESCE(invoice_number, ''), currency::text, tax_minor::text",
            "issued_at DESC NULLS LAST, id DESC",
            vec!["invoice_id", "invoice_number", "currency", "tax_minor"],
        ),
        FinanceExportType::Ledger => (
            "billing_ledger_entries",
            "id::text, entry_type::text, source_type, source_id::text, account_code, currency::text, amount_minor::text",
            "occurred_at DESC, id DESC",
            vec![
                "ledger_entry_id",
                "entry_type",
                "source_type",
                "source_id",
                "account_code",
                "currency",
                "amount_minor",
            ],
        ),
        FinanceExportType::Customers => (
            "billing_accounts",
            "id::text, workspace_id::text, COALESCE(legal_name, ''), COALESCE(billing_email, ''), currency::text, status",
            "created_at DESC, id DESC",
            vec![
                "billing_account_id",
                "workspace_id",
                "legal_name",
                "billing_email",
                "currency",
                "status",
            ],
        ),
        FinanceExportType::Subscriptions => (
            "billing_subscriptions",
            "id::text, COALESCE(workspace_id::text, ''), status, interval_unit",
            "updated_at DESC, id DESC",
            vec!["subscription_id", "workspace_id", "status", "interval_unit"],
        ),
    };
    let sql =
        format!("SELECT {columns} FROM {table} WHERE tenant_id = $1 ORDER BY {order} LIMIT 5000");
    let rows = sqlx::query(&sql).bind(tenant_id).fetch_all(db).await?;
    Ok((headers, rows.into_iter().map(row_columns).collect()))
}

fn row_columns(row: sqlx::postgres::PgRow) -> FinanceExportRow {
    FinanceExportRow {
        columns: (0..row.len())
            .map(|index| row.try_get::<String, _>(index).unwrap_or_default())
            .collect(),
    }
}
