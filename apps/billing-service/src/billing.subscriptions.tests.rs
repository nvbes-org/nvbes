use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::apply_subscription_event;

#[sqlx::test(migrations = "./migrations")]
async fn applies_subscription_created_and_skips_stale_events(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let customer_id = format!("cus_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'team', $2, 'sub@t.test')",
    )
    .bind(account_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("customer");

    let newer = Utc::now();
    let older = newer - chrono::Duration::minutes(5);

    apply_subscription_event(
        &pool,
        "evt_new",
        "customer.subscription.created",
        newer,
        &json!({
            "id": sub_id,
            "customer": customer_id,
            "status": "active",
            "metadata": { "plan_code": "standard_monthly" },
            "cancel_at_period_end": false
        }),
    )
    .await
    .expect("apply newer");

    apply_subscription_event(
        &pool,
        "evt_old",
        "customer.subscription.updated",
        older,
        &json!({
            "id": sub_id,
            "customer": customer_id,
            "status": "past_due",
            "metadata": { "plan_code": "standard_monthly" }
        }),
    )
    .await
    .expect("stale ignored");

    let status: String = sqlx::query_scalar(
        "SELECT status FROM billing_subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(&sub_id)
    .fetch_one(&pool)
    .await
    .expect("status");
    assert_eq!(status, "active");

    let outbox: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE aggregate_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("outbox");
    assert!(outbox >= 1);
}
