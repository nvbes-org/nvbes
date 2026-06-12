use axum::Router;

use crate::app::AppState;

#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;
#[path = "identity.domains.developer.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
