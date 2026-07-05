use crate::app::AppState;
use axum::{Router, middleware};

#[path = "drive.domains.billing.routes.entitlements.rs"]
mod entitlements;

pub fn router(state: &AppState) -> Router<AppState> {
    entitlements::router().layer(middleware::from_fn_with_state(
        state.config.clone(),
        nvbes_core::http::internal_observability::internal_observability_guard,
    ))
}
