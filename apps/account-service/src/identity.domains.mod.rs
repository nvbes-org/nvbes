use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.audit.rs"]
pub mod audit;
#[path = "identity.domains.auth.mod.rs"]
pub mod auth;
#[path = "identity.domains.authz.mod.rs"]
pub mod authz;
#[path = "identity.domains.legal.mod.rs"]
pub mod legal;
#[path = "identity.domains.oauth.mod.rs"]
pub mod oauth;
#[path = "identity.domains.security.mod.rs"]
pub mod security;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(auth::routes::router(state))
        .merge(authz::routes::router(state))
        .merge(security::routes::router(state))
        .merge(legal::routes::router(state))
        .merge(crate::email::routes::router(state))
}
