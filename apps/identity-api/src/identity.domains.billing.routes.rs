use crate::app::AppState;
use axum::{Router, middleware};

#[path = "identity.domains.billing.routes.manage.rs"]
pub mod manage;
#[path = "identity.domains.billing.routes.portal.rs"]
pub mod portal;
#[path = "identity.domains.billing.routes.usage.rs"]
pub mod usage;
#[path = "identity.domains.billing.routes.webhooks.rs"]
pub mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    let internal_routes = usage::router().layer(middleware::from_fn_with_state(
        state.config.clone(),
        nvbes_core::http::internal_observability::internal_observability_guard,
    ));

    Router::new()
        .merge(manage::router(state))
        .merge(portal::router())
        .merge(internal_routes)
        .merge(webhooks::router(state))
}
