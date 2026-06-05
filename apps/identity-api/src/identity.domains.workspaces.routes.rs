use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.workspaces.routes.manage.rs"]
pub mod manage;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().merge(manage::router(state))
}
