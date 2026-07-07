use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.privacy.routes.account.rs"]
mod account;
#[path = "drive.domains.privacy.routes.workspace.rs"]
mod workspace;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(account::router(state))
        .merge(workspace::router(state))
}
