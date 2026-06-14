use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.auth.mod.rs"]
pub mod auth;
#[path = "identity.domains.authz.mod.rs"]
pub mod authz;
#[path = "identity.domains.billing.mod.rs"]
pub mod billing;
#[path = "identity.domains.developer.mod.rs"]
pub mod developer;
#[path = "identity.domains.federation.mod.rs"]
pub mod federation;
#[path = "identity.domains.legal.mod.rs"]
pub mod legal;
#[path = "identity.domains.members.mod.rs"]
pub mod members;
#[path = "identity.domains.oauth.mod.rs"]
pub mod oauth;
#[path = "identity.domains.security.mod.rs"]
pub mod security;
#[path = "identity.domains.service_accounts.mod.rs"]
pub mod service_accounts;
#[path = "identity.domains.tenants.mod.rs"]
pub mod tenants;
#[path = "identity.domains.workspaces.mod.rs"]
pub mod workspaces;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(auth::routes::router(state))
        .merge(authz::routes::router(state))
        .merge(federation::routes::router(state))
        .merge(service_accounts::routes::router(state))
        .merge(tenants::routes::router(state))
        .merge(workspaces::routes::router(state))
        .merge(members::routes::router(state))
        .merge(security::routes::router(state))
        .merge(billing::routes::router(state))
        .merge(developer::routes::router(state))
        .merge(legal::routes::router(state))
        .merge(crate::email::routes::router(state))
}
