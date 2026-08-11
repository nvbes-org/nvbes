use crate::app::AppState;

#[path = "identity.worker.account_projection.client.rs"]
pub mod client;
#[path = "identity.worker.account_projection.db.rs"]
mod db;

const QUEUE_NAME: &str = "account_registration_projection";

pub async fn process_next(state: &AppState) -> anyhow::Result<bool> {
    process_next_with(&state.db, &state.account_projection).await
}

async fn process_next_with(
    database: &sqlx::PgPool,
    account_projection: &client::AccountProjectionClient,
) -> anyhow::Result<bool> {
    let Some(event) = db::claim(database).await? else {
        return Ok(false);
    };
    match account_projection.dispatch(&event.payload).await {
        Ok(()) => {
            db::complete(database, event.event_id).await?;
            tracing::info!(event_id = %event.event_id, "Account registration projection delivered");
        }
        Err(error) => {
            db::fail(database, event.event_id, error.is_retryable(), error.code()).await?;
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
    refresh_metrics_with(&state.db, &state.observability).await
}

async fn refresh_metrics_with(
    database: &sqlx::PgPool,
    observability: &nvbes_observability::metrics::HttpMetrics,
) -> anyhow::Result<()> {
    let status = db::status(database).await?;
    observability.record_worker_queue_depth(
        QUEUE_NAME,
        "pending",
        status.pending_depth,
        status.pending_oldest_age_seconds,
    );
    observability.record_worker_queue_depth(
        QUEUE_NAME,
        "dead_letter",
        status.dead_letter_depth,
        status.dead_letter_oldest_age_seconds,
    );
    Ok(())
}

#[cfg(test)]
#[path = "identity.worker.account_projection.tests.rs"]
mod tests;
