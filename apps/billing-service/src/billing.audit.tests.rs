use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::record_audit_event;

#[sqlx::test(migrations = "./migrations")]
async fn records_audit_row_against_v1_schema(pool: PgPool) {
    let account_id = Uuid::new_v4();
    record_audit_event(
        &pool,
        account_id,
        "test_actor",
        "checkout_started",
        &json!({ "plan": "standard_monthly" }),
    )
    .await
    .expect("insert");

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_audit_events WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}
