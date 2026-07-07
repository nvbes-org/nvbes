use anyhow::Context;
use serde_json::Value;

use crate::worker::{
    BillingWorkerState, analytics::capture_billing_webhook_analytics,
    email::enqueue_billing_email_for_stripe_event,
    workspace_updates::publish_workspace_billing_updates,
};

pub async fn process_stripe_webhook_job(
    state: &BillingWorkerState,
    job: &nvbes_redis::worker_queue::QueuedJob,
) -> anyhow::Result<Value> {
    let job_payload: nvbes_billing::StripeWebhookEvent =
        serde_json::from_value(job.payload.clone())
            .context("Invalid Stripe webhook job payload")?;
    let mut tx = state.db.begin().await?;
    let workspace_id =
        nvbes_billing::stripe_webhook_processing::process_stripe_event(&mut tx, &job_payload)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Stripe webhook processing failed: {e:?}")))?;
    tx.commit().await?;

    if let Some(workspace_id) = workspace_id {
        let plan_code = publish_workspace_billing_updates(state, workspace_id).await?;
        capture_billing_webhook_analytics(state, workspace_id, &job_payload, plan_code.as_deref());
        enqueue_billing_email_for_stripe_event(&state.db, &state.redis, workspace_id, &job_payload)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Billing email enqueue failed: {e:?}")))?;
    }

    Ok(serde_json::json!({"status": "processed"}))
}
