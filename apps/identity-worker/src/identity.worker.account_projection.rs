use crate::app::AppState;

#[path = "identity.worker.account_projection.client.rs"]
pub mod client;
#[path = "identity.worker.account_projection.db.rs"]
mod db;

const QUEUE_NAME: &str = "account_registration_projection";

pub async fn process_next(state: &AppState) -> anyhow::Result<bool> {
    let Some(event) = db::claim(&state.db).await? else {
        return Ok(false);
    };
    match state.account_projection.dispatch(&event.payload).await {
        Ok(()) => {
            db::complete(&state.db, event.event_id).await?;
            tracing::info!(event_id = %event.event_id, "Account registration projection delivered");
        }
        Err(error) => {
            db::fail(
                &state.db,
                event.event_id,
                error.is_retryable(),
                error.code(),
            )
            .await?;
            tracing::warn!(
                event_id = %event.event_id,
                retryable = error.is_retryable(),
                code = error.code(),
                "Account registration projection delivery failed"
            );
        }
    }
    Ok(true)
}

pub async fn refresh_metrics(state: &AppState) -> anyhow::Result<()> {
    let status = db::status(&state.db).await?;
    state.observability.record_worker_queue_depth(
        QUEUE_NAME,
        "pending",
        status.pending_depth,
        status.pending_oldest_age_seconds,
    );
    state.observability.record_worker_queue_depth(
        QUEUE_NAME,
        "dead_letter",
        status.dead_letter_depth,
        status.dead_letter_oldest_age_seconds,
    );
    Ok(())
}
