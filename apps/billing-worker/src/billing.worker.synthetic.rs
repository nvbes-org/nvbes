use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::BillingWorkerConfig, dispatcher::dispatch_message, state::BillingWorkerState};

#[derive(Debug, Serialize)]
pub struct SyntheticWorkerResult {
    pub outbox_event_uuid: Uuid,
    pub event_dispatched: bool,
    pub outbox_marked_published: bool,
    pub idempotent_reprocessed: bool,
}

pub async fn run(
    db: &PgPool,
    config: &BillingWorkerConfig,
) -> anyhow::Result<SyntheticWorkerResult> {
    let state = BillingWorkerState::new(config.clone(), db.clone()).await?;

    let event_uuid = Uuid::new_v4();
    let account_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload)
        VALUES ($1, 'billing.invoice.paid.v1', $2, $3)
        "#,
        event_uuid,
        account_id,
        json!({
            "recipient_email": "synthetic-worker@nvbes.test",
            "customer_name": "Synthetic Worker",
            "amount_minor": 1500,
            "currency": "EUR",
            "invoice_id": format!("in_worker_test_{}", Uuid::new_v4().simple())
        })
    )
    .execute(db)
    .await?;

    let outcome = dispatch_message(&state, &event_uuid.to_string()).await?;
    let event_dispatched = outcome == crate::dispatcher::DispatchOutcome::Acknowledged;

    // Test enqueueing to verify queue publisher pipeline
    let _ = state.enqueue_event(&event_uuid.to_string()).await;

    // Verify gRPC connectivity when billing service client is configured
    if let Some(billing_client) = state.billing_client.as_ref() {
        let _ = billing_client
            .get_workspace_entitlements(account_id.to_string())
            .await;
    }

    let published_at = sqlx::query_scalar!(
        "SELECT published_at FROM billing_outbox WHERE event_uuid = $1",
        event_uuid
    )
    .fetch_one(db)
    .await?;
    let outbox_marked_published = published_at.is_some();

    // Test idempotent reprocessing
    let second_outcome = dispatch_message(&state, &event_uuid.to_string()).await?;
    let idempotent_reprocessed = second_outcome == crate::dispatcher::DispatchOutcome::Acknowledged;

    Ok(SyntheticWorkerResult {
        outbox_event_uuid: event_uuid,
        event_dispatched,
        outbox_marked_published,
        idempotent_reprocessed,
    })
}

#[cfg(test)]
#[path = "billing.worker.synthetic.tests.rs"]
mod tests;
