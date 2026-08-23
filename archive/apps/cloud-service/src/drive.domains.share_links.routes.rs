use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.share_links.routes.manage.rs"]
mod manage;
#[path = "drive.domains.share_links.routes.public.rs"]
mod public;

use super::types::*;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(manage::router(state))
        .merge(public::router(state))
}
