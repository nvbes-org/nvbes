use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};

use crate::{dispatcher, state::BillingWorkerState};

pub fn router(state: BillingWorkerState) -> Router {
    Router::new()
        .route("/internal/queue/billing-dispatch", post(dispatch))
        .route("/internal/reconciliation", post(reconcile))
        .with_state(state)
}

async fn dispatch(State(state): State<BillingWorkerState>, body: Bytes) -> StatusCode {
    let payload_str = match std::str::from_utf8(&body) {
        Ok(s) => s.trim(),
        Err(_) => {
            tracing::warn!("billing queue trigger received invalid utf-8 payload");
            return StatusCode::UNPROCESSABLE_ENTITY;
        }
    };

    if payload_str.is_empty() {
        return StatusCode::NO_CONTENT;
    }

    match dispatcher::dispatch_message(&state, payload_str).await {
        Ok(dispatcher::DispatchOutcome::Acknowledged) => StatusCode::NO_CONTENT,
        Ok(dispatcher::DispatchOutcome::Retry) => StatusCode::SERVICE_UNAVAILABLE,
        Err(error) => {
            tracing::error!(payload = %payload_str, error = ?error, "billing queue trigger failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn reconcile(State(state): State<BillingWorkerState>) -> StatusCode {
    match dispatcher::sweep_pending(&state).await {
        Ok(count) => {
            tracing::info!(
                count,
                "periodic billing worker reconciliation sweep completed"
            );
            StatusCode::NO_CONTENT
        }
        Err(error) => {
            tracing::error!(error = ?error, "periodic billing worker reconciliation sweep failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

#[cfg(test)]
#[path = "billing.worker.queue.trigger.tests.rs"]
mod tests;
