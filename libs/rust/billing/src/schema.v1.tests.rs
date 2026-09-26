//! Schema contract against the active Billing V1 migrations
//! (`apps/billing-service/migrations`). Exercises only V1 tables — never Cloud
//! orphans (`billing_accounts`, `workspaces`, …).
//!
//! Uses `DATABASE_URL` (`nvbes_coverage_test`) via `#[sqlx::test]`, not the
//! `database-tests` Cargo feature (app-only gate). See
//! `apps/billing-service/src/billing.database.test_support.rs`.

use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../../apps/billing-service/migrations")]
async fn v1_foundation_tables_accept_roundtrip_rows(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let customer_id = format!("cus_lib_{}", Uuid::new_v4().simple());
    let session_id = format!("cs_lib_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_lib_{}", Uuid::new_v4().simple());
    let event_id = format!("evt_lib_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'principal', $2, 'lib@t.test')",
    )
    .bind(account_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("billing_customers");

    sqlx::query(
        "INSERT INTO billing_checkout_sessions (
            idempotency_key, account_id, account_type, plan_code,
            stripe_session_id, stripe_customer_id, checkout_url
         ) VALUES ($1, $2, 'principal', 'free', $3, $4, 'https://nvbes.test/c')",
    )
    .bind(format!("idem_{}", Uuid::new_v4().simple()))
    .bind(account_id)
    .bind(&session_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("billing_checkout_sessions");

    sqlx::query(
        "INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, monthly_price_cents
         ) VALUES ($1, 'principal', $2, $3, 'standard_monthly', 'active', 1000)",
    )
    .bind(account_id)
    .bind(&sub_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .expect("billing_subscriptions + mrr column");

    sqlx::query(
        "INSERT INTO billing_webhook_events (event_id, event_type, stripe_created_at, payload, status)
         VALUES ($1, 'invoice.paid', clock_timestamp(), '{}'::jsonb, 'processed')",
    )
    .bind(&event_id)
    .execute(&pool)
    .await
    .expect("billing_webhook_events");

    sqlx::query(
        "INSERT INTO billing_reconciliation_items (account_id, reason)
         VALUES ($1, 'schema_smoke')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("billing_reconciliation_items");

    sqlx::query(
        "INSERT INTO billing_audit_events (account_id, actor, action)
         VALUES ($1, 'schema_smoke', 'ping')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("billing_audit_events");

    sqlx::query(
        "INSERT INTO billing_outbox (event_type, aggregate_id, payload)
         VALUES ('billing.schema.smoke.v1', $1, '{}'::jsonb)",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("billing_outbox");

    let plans: i64 = sqlx::query_scalar("SELECT count(*) FROM billing_plans WHERE is_active")
        .fetch_one(&pool)
        .await
        .expect("plans seed");
    assert!(plans >= 3);

    let mrr: i64 = sqlx::query_scalar(
        "SELECT monthly_price_cents FROM billing_subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(&sub_id)
    .fetch_one(&pool)
    .await
    .expect("mrr");
    assert_eq!(mrr, 1000);
}
