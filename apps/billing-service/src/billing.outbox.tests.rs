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

#[sqlx::test(migrations = "./migrations")]
async fn publishes_invoice_notification_event_types(pool: PgPool) {
    let account_id = Uuid::new_v4();
    for (event_type, payload) in [
        (
            "billing.invoice.paid.v1",
            json!({
                "recipient_email": "paid@t.test",
                "amount_minor": 1200,
                "currency": "EUR",
                "invoice_id": "in_paid"
            }),
        ),
        (
            "billing.invoice.payment_failed.v1",
            json!({
                "recipient_email": "failed@t.test",
                "amount_minor": 900,
                "currency": "EUR",
                "invoice_id": "in_failed"
            }),
        ),
        ("billing.custom.event.v1", json!({ "ignored": true })),
    ] {
        record_outbox_event(&pool, event_type, account_id, &payload)
            .await
            .expect("record");
    }

    let published = publish_pending_outbox_events(&pool, None, "https://nvbes.test", 10)
        .await
        .expect("publish");
    assert_eq!(published, 3);
}
