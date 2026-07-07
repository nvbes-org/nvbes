use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.auth.routes.login.rs"]
pub mod login;
#[path = "identity.domains.auth.routes.mfa.rs"]
pub mod mfa;
#[path = "identity.domains.auth.routes.password.rs"]
pub mod password;
#[path = "identity.domains.auth.routes.pow.rs"]
pub mod pow;
#[path = "identity.domains.auth.routes.register.rs"]
pub mod register;
#[path = "identity.domains.auth.routes.session_mgmt.rs"]
pub mod session_mgmt;
#[path = "identity.domains.auth.routes.verification.rs"]
pub mod verification;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(super::routes_jwks::router())
        .nest("/auth", register::router())
        .nest("/auth", verification::router())
        .nest("/auth", pow::router())
        .nest("/auth", login::router())
        .nest("/auth", session_mgmt::router(state))
        .nest("/auth/mfa", mfa::router(state))
        .nest("/auth", password::router(state))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::no_cache_headers,
        ))
}
