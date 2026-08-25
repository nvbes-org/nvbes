use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};
use uuid::Uuid;

use crate::{dispatcher, error_reporting, state::EmailWorkerState};

pub fn router(state: EmailWorkerState) -> Router {
    Router::new()
        .route("/internal/queue/email-dispatch", post(dispatch))
        .with_state(state)
}

async fn dispatch(State(state): State<EmailWorkerState>, body: Bytes) -> StatusCode {
    let Some(message_id) = parse_message_id(&body) else {
        tracing::warn!("email queue trigger rejected an invalid message identifier");
        return StatusCode::UNPROCESSABLE_ENTITY;
    };

    match dispatcher::dispatch_message(&state, message_id).await {
        Ok(dispatcher::DispatchOutcome::Acknowledged) => StatusCode::NO_CONTENT,
        Ok(dispatcher::DispatchOutcome::Retry) => StatusCode::SERVICE_UNAVAILABLE,
        Err(error) => {
            error_reporting::capture_operation(&state.config, "queue_dispatch", error.as_ref());
            tracing::error!(%message_id, error = ?error, "email queue trigger failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

fn parse_message_id(body: &[u8]) -> Option<Uuid> {
    std::str::from_utf8(body)
        .ok()
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

#[cfg(test)]
#[path = "email.worker.queue.trigger.tests.rs"]
mod tests;
