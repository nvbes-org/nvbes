use std::sync::{Arc, OnceLock};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::state::BillingWorkerState;

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("Billing Worker Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn router(state: BillingWorkerState) -> Router {
    Router::new()
        .route("/metrics", get(render))
        .with_state(state)
}

async fn render(State(state): State<BillingWorkerState>, headers: HeaderMap) -> impl IntoResponse {
    let handle = install();
    let expected = match &state.config.metrics_token {
        Some(token) if !token.is_empty() => token.as_bytes(),
        _ => return (StatusCode::UNAUTHORIZED, String::new()),
    };

    let provided = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();

    if !nvbes_billing::stripe::constant_time_eq(provided.as_bytes(), expected) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }
    (StatusCode::OK, handle.render())
}

#[cfg(test)]
#[path = "billing.worker.metrics.tests.rs"]
mod tests;
