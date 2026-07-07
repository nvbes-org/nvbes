use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.uploads.routes.core.rs"]
mod core;
#[path = "drive.domains.uploads.routes.tus.rs"]
mod tus;

use super::service;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().merge(core::router(state))
}
