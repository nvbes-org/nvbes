use crate::{database, dispatch_db, email_metrics, error_reporting, state::EmailWorkerState};

pub async fn suppress_due_messages(state: &EmailWorkerState) -> bool {
    match dispatch_db::suppress_due_messages(&state.db).await {
        Ok(suppressed) => {
            email_metrics::suppressed(suppressed);
            true
        }
        Err(error) => {
            error_reporting::capture_operation(&state.config, "suppression_sweep", &error);
            tracing::error!(error = ?error, "email suppression sweep failed");
            false
        }
    }
}

pub async fn expire_stale_messages(state: &EmailWorkerState) {
    match database::expire_stale_messages(&state.db).await {
        Ok(expired) if expired > 0 => {
            email_metrics::expired(expired);
            tracing::info!(expired, "expired stale email commands");
        }
        Ok(_) => {}
        Err(error) => {
            error_reporting::capture_operation(&state.config, "deadline_sweep", &error);
            tracing::error!(error = ?error, "email deadline sweep failed");
        }
    }
}

pub async fn record_queue_metrics(state: &EmailWorkerState) {
    match database::queue_snapshot(&state.db).await {
        Ok((depth, oldest_age)) => email_metrics::queue(depth, oldest_age),
        Err(error) => {
            error_reporting::capture_operation(&state.config, "queue_metrics", &error);
            tracing::error!(error = ?error, "email queue metrics query failed");
        }
    }
}

pub async fn apply_retention(state: &EmailWorkerState) {
    let policy = &state.config.retention;
    match database::apply_retention(&state.db, policy.payload_days, policy.ledger_days).await {
        Ok(result) => email_metrics::retention(
            result.payloads,
            result.diagnostics,
            result.messages,
            result.events,
        ),
        Err(error) => {
            error_reporting::capture_operation(&state.config, "retention_sweep", &error);
            tracing::error!(error = ?error, "email retention sweep failed");
        }
    }
}
