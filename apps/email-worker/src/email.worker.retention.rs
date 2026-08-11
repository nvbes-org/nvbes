use axum::{Router, extract::State, http::StatusCode, routing::post};

use crate::{database, dispatch_db, email_metrics, state::EmailWorkerState};

pub fn router(state: EmailWorkerState) -> Router {
    Router::new()
        .route("/internal/retention", post(run))
        .with_state(state)
}

async fn run(State(state): State<EmailWorkerState>) -> StatusCode {
    if let Err(error) = retain(&state).await {
        tracing::error!(error = ?error, "email retention trigger failed");
        return StatusCode::SERVICE_UNAVAILABLE;
    }
    StatusCode::NO_CONTENT
}

async fn retain(state: &EmailWorkerState) -> anyhow::Result<()> {
    let suppressed = dispatch_db::suppress_due_messages(&state.db).await?;
    email_metrics::suppressed(suppressed);
    let expired = database::expire_stale_messages(&state.db).await?;
    email_metrics::expired(expired);
    let policy = &state.config.retention;
    let result =
        database::apply_retention(&state.db, policy.payload_days, policy.ledger_days).await?;
    email_metrics::retention(
        result.payloads,
        result.diagnostics,
        result.messages,
        result.events,
    );
    Ok(())
}
