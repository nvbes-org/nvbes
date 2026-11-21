use std::sync::{Arc, OnceLock};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::app::BillingState;

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("Billing Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn router() -> Router<BillingState> {
    Router::new().route("/metrics", get(render))
}

async fn render(State(state): State<BillingState>, headers: HeaderMap) -> impl IntoResponse {
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
    (StatusCode::OK, state.metrics.render())
}

#[cfg(test)]
#[path = "billing.metrics.tests.rs"]
mod tests;
