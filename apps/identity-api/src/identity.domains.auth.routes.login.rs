use crate::app::AppState;
use axum::Router;

#[path = "identity.domains.auth.routes.login.identifier.rs"]
pub mod identifier;
#[path = "identity.domains.auth.routes.login.mfa.rs"]
pub mod mfa;
#[path = "identity.domains.auth.routes.login.pwd.rs"]
pub mod pwd;
#[path = "identity.domains.auth.routes.login.webauthn.rs"]
pub mod webauthn;

#[path = "identity.domains.auth.routes.login.types.rs"]
mod types;

#[derive(serde::Deserialize)]
pub(crate) struct LoginQuery {
    pub(crate) authuser: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(identifier::router())
        .merge(pwd::router())
        .merge(webauthn::router())
        .merge(mfa::router())
}
