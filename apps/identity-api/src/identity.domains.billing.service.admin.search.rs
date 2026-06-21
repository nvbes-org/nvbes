use serde::Serialize;

use crate::http::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BillingAdminSearchResult {
    pub entity_type: String,
    pub entity_id: uuid::Uuid,
    pub label: String,
    pub status: String,
}

pub async fn search_billing_admin(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    query: &str,
) -> Result<Vec<BillingAdminSearchResult>, AppError> {
    let normalized = query.trim();
    if normalized.len() < 2 {
        return Err(AppError::bad_request(
            "billing_search_query_too_short",
            "Billing admin search query must contain at least two characters.",
        ));
    }

    sqlx::query_as::<_, BillingAdminSearchRow>(
        r#"
        SELECT 'invoice' AS entity_type, id AS entity_id,
               COALESCE(invoice_number, id::text) AS label, status::text AS status
        FROM billing_invoices
        WHERE tenant_id = $1
          AND (invoice_number ILIKE '%' || $2 || '%' OR id::text = $2)
        UNION ALL
        SELECT 'payment' AS entity_type, id AS entity_id,
               id::text AS label, status::text AS status
        FROM billing_payments
        WHERE tenant_id = $1
          AND id::text = $2
        UNION ALL
        SELECT 'customer' AS entity_type, id AS entity_id,
               COALESCE(legal_name, billing_email, id::text) AS label, status
        FROM billing_accounts
        WHERE tenant_id = $1
          AND (
            legal_name ILIKE '%' || $2 || '%'
            OR billing_email ILIKE '%' || $2 || '%'
            OR id::text = $2
          )
        UNION ALL
        SELECT 'subscription' AS entity_type, id AS entity_id,
               id::text AS label, status
        FROM billing_subscriptions
        WHERE tenant_id = $1
          AND (id::text = $2 OR workspace_id::text = $2)
        LIMIT 50
        "#,
    )
    .bind(tenant_id)
    .bind(normalized)
    .fetch_all(db)
    .await
    .map(|rows| rows.into_iter().map(Into::into).collect())
    .map_err(AppError::from)
}

#[derive(Debug, sqlx::FromRow)]
struct BillingAdminSearchRow {
    entity_type: String,
    entity_id: uuid::Uuid,
    label: String,
    status: String,
}

impl From<BillingAdminSearchRow> for BillingAdminSearchResult {
    fn from(row: BillingAdminSearchRow) -> Self {
        Self {
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            label: row.label,
            status: row.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_admin_search_result_is_provider_neutral() {
        let result = BillingAdminSearchResult {
            entity_type: "invoice".to_string(),
            entity_id: uuid::Uuid::nil(),
            label: "INV-2026-0001".to_string(),
            status: "issued".to_string(),
        };

        assert_eq!(result.entity_type, "invoice");
        assert!(!format!("{result:?}").contains("stripe"));
    }
}
