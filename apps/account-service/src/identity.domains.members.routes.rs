use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.members.routes.invitations.rs"]
pub mod invitations;
#[path = "identity.domains.members.routes.manage.rs"]
pub mod manage;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(manage::router(state))
        .merge(invitations::router(state))
}
