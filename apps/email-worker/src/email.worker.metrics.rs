use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::state::EmailWorkerState;

pub fn router(state: EmailWorkerState) -> Router {
    Router::new()
        .route("/metrics", get(render))
        .with_state(state)
}

async fn render(State(state): State<EmailWorkerState>) -> impl IntoResponse {
    state.metrics.render()
}

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("email Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn accepted(producer: &str, business_type: &str, template_version: i16, duplicate: bool) {
    metrics::counter!(
        "email_messages_total",
        "producer" => producer.to_string(),
        "business_type" => business_type.to_string(),
        "template_version" => template_version.to_string(),
        "outcome" => if duplicate { "duplicate" } else { "accepted" },
    )
    .increment(1);
}

pub fn queue(depth: i64, oldest_age_seconds: Option<f64>) {
    metrics::gauge!("email_queue_depth").set(depth as f64);
    metrics::gauge!("email_queue_oldest_age_seconds").set(oldest_age_seconds.unwrap_or_default());
}

pub fn dispatch(provider: &str, business_type: &str, outcome: &str, duration: Duration) {
    let labels = [
        ("provider", provider.to_string()),
        ("business_type", business_type.to_string()),
        ("outcome", outcome.to_string()),
    ];
    metrics::counter!("email_messages_total", &labels).increment(1);
    metrics::histogram!("email_delivery_duration_seconds", &labels).record(duration.as_secs_f64());
}

pub fn expired(count: u64) {
    metrics::counter!("email_expired_total").increment(count);
}

pub fn suppressed(count: u64) {
    metrics::counter!("email_suppressed_total").increment(count);
}

pub fn webhook(event_type: &str, outcome: &str, duration: Duration) {
    let labels = [
        ("provider", "scaleway".to_string()),
        ("event_type", event_type.to_string()),
        ("outcome", outcome.to_string()),
    ];
    metrics::counter!("email_provider_webhooks_total", &labels).increment(1);
    metrics::histogram!("email_provider_webhook_duration_seconds", &labels)
        .record(duration.as_secs_f64());
}

pub fn webhook_signature_failure() {
    metrics::counter!("email_webhook_signature_failures_total").increment(1);
}

pub fn retention(payloads: u64, diagnostics: u64, messages: u64, events: u64) {
    for (kind, count) in [
        ("payload", payloads),
        ("diagnostic", diagnostics),
        ("message", messages),
        ("event", events),
    ] {
        metrics::counter!("email_retention_rows_total", "kind" => kind).increment(count);
    }
}

#[cfg(test)]
mod tests {
    use super::{accepted, install};

    #[test]
    fn acceptance_metrics_expose_bounded_operational_labels() {
        let handle = install();
        accepted("identity-service", "email_verification", 1, false);
        let rendered = handle.render();
        assert!(rendered.contains("email_messages_total"));
        assert!(rendered.contains("producer=\"identity-service\""));
        assert!(!rendered.contains("recipient"));
    }
}
