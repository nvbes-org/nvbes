use anyhow::Context;
use nvbes_redis::worker_queue::QueuedJob;
use serde_json::{Value, json};

use crate::worker::BillingWorkerState;

pub(crate) async fn process_billing_email_job(
    state: &BillingWorkerState,
    job: &QueuedJob,
) -> anyhow::Result<Value> {
    let command: nvbes_email::EmailCommand = serde_json::from_value(job.payload.clone())
        .context("invalid billing email command payload")?;
    let receipt = state.email.send(command).await?;
    Ok(json!({
        "status": "accepted",
        "domain": "billing",
        "message_id": receipt.message_id,
        "duplicate": receipt.duplicate,
    }))
}
