use crate::app::AppState;
use axum::Router;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
