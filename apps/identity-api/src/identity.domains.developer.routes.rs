use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
