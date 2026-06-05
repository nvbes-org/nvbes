use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.workspaces.routes.manage.rs"]
pub mod manage;

use super::service;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().merge(manage::router(state))
}
