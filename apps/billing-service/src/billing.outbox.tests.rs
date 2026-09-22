use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::{publish_pending_outbox_events, record_outbox_event};

#[sqlx::test(migrations = "./migrations")]
async fn records_and_publishes_pending_outbox(pool: PgPool) {
    let account_id = Uuid::new_v4();
    record_outbox_event(
        &pool,
        "billing.subscription.changed.v1",
        account_id,
        &json!({ "status": "active" }),
    )
    .await
    .expect("record");

    let published = publish_pending_outbox_events(&pool, None, "https://nvbes.test", 10)
        .await
        .expect("publish");
    assert_eq!(published, 1);

    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(pending, 0);
}
