use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.members.routes.invitations.rs"]
mod invitations;
#[path = "drive.domains.members.routes.manage.rs"]
mod manage;

use super::service;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(manage::router(state))
        .merge(invitations::router(state))
}
