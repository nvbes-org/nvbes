use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{Router, extract::State, middleware, response::IntoResponse, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::state::EmailWorkerState;

pub fn router(state: EmailWorkerState) -> Router {
    let auth = nvbes_core::http::internal_observability::InternalObservabilityConfig::new(
        &state.config.environment,
        state.config.observability_internal_token.as_deref(),
    );
    Router::new()
        .route("/metrics", get(render))
        .route_layer(middleware::from_fn_with_state(
            auth,
            nvbes_core::http::internal_observability::internal_observability_guard_with_config,
        ))
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
    use std::time::Duration;

    use super::{
        accepted, dispatch, expired, install, retention, suppressed, webhook,
        webhook_signature_failure,
    };

    #[test]
    fn acceptance_metrics_expose_bounded_operational_labels() {
        let handle = install();
        accepted("identity-service", "email_verification", 1, false);
        let rendered = handle.render();
        assert!(rendered.contains("email_messages_total"));
        assert!(rendered.contains("producer=\"identity-service\""));
        assert!(!rendered.contains("recipient"));
    }

    #[test]
    fn every_metric_family_can_be_recorded_without_sensitive_labels() {
        let handle = install();
        accepted("identity-service", "account_security_v1", 1, true);
        dispatch(
            "mock",
            "account_security_v1",
            "provider_accepted",
            Duration::from_millis(25),
        );
        expired(2);
        suppressed(1);
        webhook("email_delivered", "state_applied", Duration::from_millis(5));
        webhook_signature_failure();
        retention(1, 2, 3, 4);

        let rendered = handle.render();
        for metric in [
            "email_delivery_duration_seconds",
            "email_expired_total",
            "email_suppressed_total",
            "email_provider_webhooks_total",
            "email_webhook_signature_failures_total",
            "email_retention_rows_total",
        ] {
            assert!(rendered.contains(metric), "missing {metric}");
        }
    }

    #[cfg(feature = "database-tests")]
    #[sqlx::test(migrations = "./migrations")]
    async fn metrics_router_renders_the_installed_recorder(pool: sqlx::PgPool) {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let mut config = crate::test_support::config(crate::config::ProviderConfig::Mock);
        config.observability_internal_token = Some(crate::test_support::INTERNAL_TOKEN.to_string());
        let state = crate::state::EmailWorkerState::new(config, pool).unwrap();
        let response = super::router(state)
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header(
                        "x-nvbes-internal-token",
                        crate::test_support::INTERNAL_TOKEN,
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
