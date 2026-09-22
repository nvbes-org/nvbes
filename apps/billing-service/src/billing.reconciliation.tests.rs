use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::record_reconciliation_item;

#[sqlx::test(migrations = "./migrations")]
async fn records_pending_reconciliation_item(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let id = record_reconciliation_item(
        &pool,
        Some("evt_recon"),
        Some(account_id),
        "orphan_invoice",
        &json!({ "invoice_id": "in_1" }),
    )
    .await
    .expect("insert");

    let status: String =
        sqlx::query_scalar("SELECT status FROM billing_reconciliation_items WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("status");
    assert_eq!(status, "pending");
}
