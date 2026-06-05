use crate::app::AppState;
use axum::Router;

#[path = "drive.domains.files.routes.browse.rs"]
mod browse;
#[path = "drive.domains.files.routes.download.rs"]
mod download;
#[path = "drive.domains.files.routes.manage.rs"]
mod manage;
#[path = "drive.domains.files.routes.trash.rs"]
mod trash;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(browse::router(state))
        .merge(manage::router(state))
        .merge(trash::router(state))
        .merge(download::router(state))
}
