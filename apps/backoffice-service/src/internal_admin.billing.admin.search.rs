use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::SearchResult;
use crate::error::AppError;

pub(crate) async fn search_billing_admin(
    db: &PgPool,
    tenant_id: Uuid,
    query: &str,
) -> Result<Vec<SearchResult>, AppError> {
    if query.trim().len() < 2 {
        return Err(AppError::bad_request(
            "invalid_search_query",
            "Billing admin search requires at least two characters.",
        ));
    }
    let pattern = format!("%{}%", query.trim());
    let rows = sqlx::query(
        "SELECT 'invoice', id, COALESCE(invoice_number, id::text), status::text
         FROM billing_invoices
         WHERE tenant_id = $1 AND (invoice_number ILIKE $2 OR id::text ILIKE $2)
         UNION ALL
         SELECT 'payment', id, provider::text || ':' || id::text, status::text
         FROM billing_payments
         WHERE tenant_id = $1 AND id::text ILIKE $2
         ORDER BY 1, 3
         LIMIT 25",
    )
    .bind(tenant_id)
    .bind(pattern)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| SearchResult {
            kind: row.get(0),
            id: row.get(1),
            label: row.get(2),
            status: row.get(3),
        })
        .collect())
}
