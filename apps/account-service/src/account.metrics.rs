use std::sync::{Arc, OnceLock};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::app::AccountState;

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("Account Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/metrics", get(render))
        .with_state(state)
}

async fn render(State(state): State<AccountState>, headers: HeaderMap) -> impl IntoResponse {
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

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let difference = (0..max_len).fold(left.len() ^ right.len(), |difference, index| {
        difference
            | usize::from(
                left.get(index).copied().unwrap_or_default()
                    ^ right.get(index).copied().unwrap_or_default(),
            )
    });
    difference == 0
}

#[cfg(test)]
#[path = "account.metrics.tests.rs"]
mod tests;
