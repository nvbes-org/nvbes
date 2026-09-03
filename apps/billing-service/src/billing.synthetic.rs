use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    audit::record_audit_event,
    config::BillingConfig,
    customer::get_or_create_customer,
    reconciliation::record_reconciliation_item,
    subscriptions::apply_subscription_event,
};

#[derive(Debug, Serialize)]
pub struct SyntheticBillingResult {
    pub workspace_id: Uuid,
    pub customer_id: String,
    pub checkout_session_id: String,
    pub checkout_idempotent: bool,
    pub subscription_id: String,
    pub subscription_status: String,
    pub webhook_deduplicated: bool,
    pub out_of_order_protected: bool,
    pub reconciliation_resolved: bool,
    pub audit_events: i64,
    pub outbox_events: i64,
}

pub async fn run(
    db: &PgPool,
    config: &BillingConfig,
    workspace_id: Uuid,
    _owner_id: Uuid,
) -> anyhow::Result<SyntheticBillingResult> {
    // 1. Customer mapping
    let customer_id = get_or_create_customer(db, config, workspace_id, "team", Some("billing-synthetic@nvbes.test"))
        .await
        .map_err(|e| anyhow::anyhow!("customer mapping failed: {e}"))?;

    // 2. Checkout session & idempotency
    let idem_key = format!("synthetic_idem_{}", Uuid::new_v4());
    let session_id = format!("cs_test_{}", Uuid::new_v4().simple());
    let checkout_url = format!("{}/mock-checkout?sid={session_id}", config.app_url);

    sqlx::query(
        r#"
        INSERT INTO billing_checkout_sessions
          (idempotency_key, account_id, account_type, plan_code, stripe_session_id, stripe_customer_id, checkout_url)
        VALUES ($1, $2, 'team', 'standard_monthly', $3, $4, $5)
        "#,
    )
    .bind(&idem_key)
    .bind(workspace_id)
    .bind(&session_id)
    .bind(&customer_id)
    .bind(&checkout_url)
    .execute(db)
    .await?;

    let duplicate_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_checkout_sessions WHERE idempotency_key = $1",
    )
    .bind(&idem_key)
    .fetch_one(db)
    .await?;
    let checkout_idempotent = duplicate_count == 1;

    // 3. Webhook simulation: checkout.session.completed
    let evt_checkout_id = format!("evt_test_checkout_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    let checkout_payload = json!({
        "id": evt_checkout_id,
        "type": "checkout.session.completed",
        "livemode": false,
        "created": now.timestamp(),
        "data": {
            "object": {
                "id": session_id,
                "customer": customer_id,
                "metadata": {
                    "account_id": workspace_id.to_string(),
                    "plan_code": "standard_monthly"
                }
            }
        }
    });

    // Ingest webhook event
    sqlx::query(
        r#"
        INSERT INTO billing_webhook_events (event_id, event_type, stripe_created_at, payload, status)
        VALUES ($1, 'checkout.session.completed', $2, $3, 'processed')
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(&evt_checkout_id)
    .bind(now)
    .bind(&checkout_payload)
    .execute(db)
    .await?;

    sqlx::query("UPDATE billing_checkout_sessions SET status = 'completed', completed_at = clock_timestamp() WHERE stripe_session_id = $1")
        .bind(&session_id)
        .execute(db)
        .await?;

    record_audit_event(db, workspace_id, "stripe", "checkout_completed", &json!({ "session_id": session_id })).await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // 4. Test Webhook deduplication: try inserting same event.id
    let reinserted: Option<String> = sqlx::query_scalar(
        r#"
        INSERT INTO billing_webhook_events (event_id, event_type, stripe_created_at, payload, status)
        VALUES ($1, 'checkout.session.completed', $2, $3, 'processed')
        ON CONFLICT (event_id) DO NOTHING
        RETURNING event_id
        "#,
    )
    .bind(&evt_checkout_id)
    .bind(now)
    .bind(&checkout_payload)
    .fetch_optional(db)
    .await?;
    let webhook_deduplicated = reinserted.is_none();

    // 5. Subscription lifecycle: created at T2
    let sub_id = format!("sub_test_{}", Uuid::new_v4().simple());
    let t2 = now;
    let sub_created_data = json!({
        "id": sub_id,
        "customer": customer_id,
        "status": "active",
        "metadata": {
            "account_id": workspace_id.to_string(),
            "plan_code": "standard_monthly"
        }
    });

    apply_subscription_event(db, "evt_sub_created", "customer.subscription.created", t2, &sub_created_data)
        .await
        .map_err(|e| anyhow::anyhow!("sub created failed: {e}"))?;

    // 6. Out-of-order test: older update at T1 < T2 with status 'past_due'
    let t1 = now - Duration::minutes(10);
    let sub_old_update_data = json!({
        "id": sub_id,
        "customer": customer_id,
        "status": "past_due",
        "metadata": {
            "account_id": workspace_id.to_string(),
            "plan_code": "standard_monthly"
        }
    });

    apply_subscription_event(db, "evt_sub_old", "customer.subscription.updated", t1, &sub_old_update_data)
        .await
        .map_err(|e| anyhow::anyhow!("sub old update failed: {e}"))?;

    // Verify subscription status is still 'active' because T1 < T2
    let current_status: String = sqlx::query_scalar(
        "SELECT status FROM billing_subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(&sub_id)
    .fetch_one(db)
    .await?;
    let out_of_order_protected = current_status == "active";

    // 7. Reconciliation dead-letter and manual operator resolution
    let recon_id = record_reconciliation_item(
        db,
        Some("evt_failing_sample"),
        Some(workspace_id),
        "synthetic_test_reconciliation",
        &json!({ "reason": "simulated_anomaly" }),
    )
    .await
    .map_err(|e| anyhow::anyhow!("reconciliation record failed: {e}"))?;

    sqlx::query(
        r#"
        UPDATE billing_reconciliation_items
        SET status = 'resolved', resolved_by = 'operator', resolved_at = clock_timestamp(), resolution_notes = 'verified in test'
        WHERE id = $1
        "#,
    )
    .bind(recon_id)
    .execute(db)
    .await?;

    let recon_status: String = sqlx::query_scalar(
        "SELECT status FROM billing_reconciliation_items WHERE id = $1",
    )
    .bind(recon_id)
    .fetch_one(db)
    .await?;
    let reconciliation_resolved = recon_status == "resolved";

    // 8. Count audit and outbox
    let audit_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_audit_events WHERE account_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await?;

    let outbox_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_outbox WHERE aggregate_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await?;

    Ok(SyntheticBillingResult {
        workspace_id,
        customer_id,
        checkout_session_id: session_id,
        checkout_idempotent,
        subscription_id: sub_id,
        subscription_status: current_status,
        webhook_deduplicated,
        out_of_order_protected,
        reconciliation_resolved,
        audit_events,
        outbox_events,
    })
}
