use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.billing.routes.manage.rs"]
mod manage;
#[path = "drive.domains.billing.routes.webhooks.rs"]
mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(manage::router(state))
        .merge(webhooks::router(state))
}
