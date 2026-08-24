use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::{app::TrustRiskState, auth::constant_time_eq};

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("trust/risk Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn router(state: TrustRiskState) -> Router {
    Router::new()
        .route("/metrics", get(render))
        .with_state(state)
}

async fn render(State(state): State<TrustRiskState>, headers: HeaderMap) -> impl IntoResponse {
    let provided = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), state.config.metrics_token.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }
    (StatusCode::OK, state.metrics.render())
}

pub fn signal(producer: &str, family: &str, outcome: &'static str) {
    metrics::counter!("trust_risk_signals_total", "producer" => producer.to_string(), "family" => family.to_string(), "outcome" => outcome).increment(1);
}

pub fn assessment(
    operation: &str,
    recommendation: &str,
    outcome: &'static str,
    duration: Duration,
) {
    let labels = [
        ("operation", operation.to_string()),
        ("recommendation", recommendation.to_string()),
        ("outcome", outcome.to_string()),
    ];
    metrics::counter!("trust_risk_assessments_total", &labels).increment(1);
    metrics::histogram!("trust_risk_assessment_duration_seconds", &labels)
        .record(duration.as_secs_f64());
}

pub fn projection(outcome: &'static str, count: u64) {
    metrics::counter!("trust_risk_projection_events_total", "outcome" => outcome).increment(count);
}
