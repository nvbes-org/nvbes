use sqlx::PgPool;
use uuid::Uuid;

use super::fetch_account_billing_overview;

#[sqlx::test(migrations = "./migrations")]
async fn returns_none_without_subscription(pool: PgPool) {
    let overview = fetch_account_billing_overview(&pool, Uuid::new_v4())
        .await
        .expect("query");
    assert!(overview.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn loads_subscription_snapshot_from_v1_tables(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let customer_id = format!("cus_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'team', $2, 'ov@t.test')",
    )
    .bind(account_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("customer");

    sqlx::query(
        "INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, monthly_price_cents, cancel_at_period_end
         ) VALUES ($1, 'team', $2, $3, 'pro_monthly', 'active', 2500, false)",
    )
    .bind(account_id)
    .bind(&sub_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("subscription");

    let overview = fetch_account_billing_overview(&pool, account_id)
        .await
        .expect("query")
        .expect("present");
    assert_eq!(overview.plan_code, "pro_monthly");
    assert_eq!(overview.status, "active");
    assert_eq!(overview.monthly_price_cents, 2500);
    assert_eq!(overview.customer_email.as_deref(), Some("ov@t.test"));
    assert!(!overview.cancel_at_period_end);
}
