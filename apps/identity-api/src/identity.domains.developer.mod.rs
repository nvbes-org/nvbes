use axum::Router;

use crate::app::AppState;

#[path = "identity.domains.developer.apps.routes.rs"]
pub mod apps_routes;
#[path = "identity.domains.developer.apps.service.rs"]
pub mod apps_service;
#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;
#[path = "identity.domains.developer.rbac.db.rs"]
pub mod rbac_db;
#[path = "identity.domains.developer.routes.rs"]
pub mod routes;
#[path = "identity.domains.developer.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(routes::router(state))
        .merge(apps_routes::router(state))
}
