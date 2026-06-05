use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.billing.routes.manage.rs"]
pub mod manage;
#[path = "identity.domains.billing.routes.webhooks.rs"]
pub mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(manage::router(state))
        .merge(webhooks::router(state))
}
