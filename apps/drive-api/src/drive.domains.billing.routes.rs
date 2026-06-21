use crate::app::AppState;
use axum::{Router, middleware};

#[path = "drive.domains.billing.routes.entitlements.rs"]
mod entitlements;
#[path = "drive.domains.billing.routes.manage.rs"]
mod manage;
#[path = "drive.domains.billing.routes.webhooks.rs"]
mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    let internal_routes = entitlements::router().layer(middleware::from_fn_with_state(
        state.config.clone(),
        nvbes_core::http::internal_observability::internal_observability_guard,
    ));

    Router::new()
        .merge(manage::router(state))
        .merge(webhooks::router(state))
        .merge(internal_routes)
}
