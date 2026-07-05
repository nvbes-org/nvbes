use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::provider::ProviderCode;

#[derive(Debug, Clone)]
pub struct ProviderInvoiceInput {
    pub workspace_id: Uuid,
    pub provider: ProviderCode,
    pub provider_invoice_id: String,
    pub provider_invoice_number: Option<String>,
    pub provider_pdf_url: Option<String>,
    pub status: String,
    pub currency: String,
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub issued_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub metadata: Value,
    pub lines: Vec<ProviderInvoiceLineInput>,
}

#[derive(Debug, Clone)]
pub struct ProviderInvoiceLineInput {
    pub provider_line_id: Option<String>,
    pub line_type: String,
    pub description: String,
    pub quantity: i64,
    pub currency: String,
    pub unit_amount_minor: i64,
    pub amount_minor: i64,
    pub tax_minor: i64,
    pub metadata: Value,
}

pub async fn upsert_provider_invoice_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: ProviderInvoiceInput,
) -> Result<(), sqlx::Error> {
    let invoice_id = upsert_invoice_header_tx(tx, &input).await?;
    if !input.lines.is_empty() {
        replace_invoice_lines_tx(tx, invoice_id, &input.lines).await?;
    }
    upsert_provider_invoice_link_tx(tx, invoice_id, &input).await
}

async fn upsert_invoice_header_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &ProviderInvoiceInput,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        WITH billing_account AS (
          SELECT id AS billing_account_id, tenant_id
          FROM billing_accounts
          WHERE workspace_id = $1
          LIMIT 1
        ),
        subscription AS (
          SELECT id
          FROM billing_subscriptions
          WHERE workspace_id = $1
          ORDER BY updated_at DESC, created_at DESC
          LIMIT 1
        ),
        existing_provider_invoice AS (
          SELECT invoice_id
          FROM billing_provider_invoices
          WHERE provider = $2::billing_provider
            AND provider_invoice_id = $3
          LIMIT 1
        ),
        inserted_invoice AS (
          INSERT INTO billing_invoices (
            tenant_id,
            billing_account_id,
            subscription_id,
            invoice_number,
            status,
            currency,
            subtotal_minor,
            tax_minor,
            total_minor,
            issued_at,
            due_at,
            paid_at
          )
          SELECT ba.tenant_id,
                 ba.billing_account_id,
                 subscription.id,
                 $4,
                 $5::billing_invoice_status,
                 $6,
                 $7,
                 $8,
                 $9,
                 $10,
                 $11,
                 $12
          FROM billing_account ba
          LEFT JOIN subscription ON TRUE
          WHERE NOT EXISTS (SELECT 1 FROM existing_provider_invoice)
          RETURNING id
        ),
        selected_invoice AS (
          SELECT invoice_id AS id FROM existing_provider_invoice
          UNION ALL
          SELECT id FROM inserted_invoice
          LIMIT 1
        )
        UPDATE billing_invoices invoice
        SET subscription_id = COALESCE(subscription.id, invoice.subscription_id),
            invoice_number = COALESCE($4, invoice.invoice_number),
            status = $5::billing_invoice_status,
            currency = $6,
            subtotal_minor = $7,
            tax_minor = $8,
            total_minor = $9,
            issued_at = COALESCE($10, invoice.issued_at),
            due_at = COALESCE($11, invoice.due_at),
            paid_at = COALESCE($12, invoice.paid_at),
            updated_at = NOW()
        FROM selected_invoice
        LEFT JOIN subscription ON TRUE
        WHERE invoice.id = selected_invoice.id
        RETURNING invoice.id
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.provider.as_str())
    .bind(&input.provider_invoice_id)
    .bind(&input.provider_invoice_number)
    .bind(&input.status)
    .bind(&input.currency)
    .bind(input.subtotal_minor)
    .bind(input.tax_minor)
    .bind(input.total_minor)
    .bind(input.issued_at)
    .bind(input.due_at)
    .bind(input.paid_at)
    .fetch_one(tx.as_mut())
    .await
}

async fn replace_invoice_lines_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice_id: Uuid,
    lines: &[ProviderInvoiceLineInput],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM billing_invoice_lines WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(tx.as_mut())
        .await?;

    for line in lines {
        sqlx::query(
            r#"
            INSERT INTO billing_invoice_lines (
              invoice_id,
              line_type,
              description,
              quantity,
              currency,
              unit_amount_minor,
              amount_minor,
              tax_minor,
              metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(invoice_id)
        .bind(&line.line_type)
        .bind(&line.description)
        .bind(line.quantity)
        .bind(&line.currency)
        .bind(line.unit_amount_minor)
        .bind(line.amount_minor)
        .bind(line.tax_minor)
        .bind(line_metadata(line))
        .execute(tx.as_mut())
        .await?;
    }

    Ok(())
}

async fn upsert_provider_invoice_link_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice_id: Uuid,
    input: &ProviderInvoiceInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO billing_provider_invoices (
          tenant_id,
          invoice_id,
          provider,
          provider_invoice_id,
          provider_invoice_number,
          provider_pdf_url,
          status,
          metadata
        )
        SELECT tenant_id,
               id,
               $2::billing_provider,
               $3,
               $4,
               $5,
               $6,
               $7
        FROM billing_invoices
        WHERE id = $1
        ON CONFLICT (provider, provider_invoice_id) DO UPDATE
        SET invoice_id = EXCLUDED.invoice_id,
            provider_invoice_number = EXCLUDED.provider_invoice_number,
            provider_pdf_url = EXCLUDED.provider_pdf_url,
            status = EXCLUDED.status,
            metadata = EXCLUDED.metadata,
            updated_at = NOW()
        "#,
    )
    .bind(invoice_id)
    .bind(input.provider.as_str())
    .bind(&input.provider_invoice_id)
    .bind(&input.provider_invoice_number)
    .bind(&input.provider_pdf_url)
    .bind(&input.status)
    .bind(&input.metadata)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn line_metadata(line: &ProviderInvoiceLineInput) -> Value {
    let mut metadata = match line.metadata.clone() {
        Value::Object(object) => object,
        _ => serde_json::Map::new(),
    };
    if let Some(provider_line_id) = line.provider_line_id.clone() {
        metadata.insert(
            "provider_line_id".to_string(),
            Value::String(provider_line_id),
        );
    }
    Value::Object(metadata)
}
