#[path = "identity.domains.auth.sessions.activity.rs"]
mod activity;
#[path = "identity.domains.auth.sessions.authenticate.rs"]
mod authenticate;
#[path = "identity.domains.auth.sessions.browser.rs"]
mod browser;
#[path = "identity.domains.auth.sessions.authenticate.browser.rs"]
mod browser_authenticate;
#[path = "identity.domains.auth.sessions.cache.rs"]
pub mod cache;
#[path = "identity.domains.auth.sessions.cookie_theft.rs"]
pub mod cookie_theft;
#[path = "identity.domains.auth.sessions.create.rs"]
mod create;
#[path = "identity.domains.auth.sessions.token.rs"]
pub mod token;

pub use super::sessions_mgmt::{
    fetch_view, list, logout, revoke, revoke_all_others, revoke_all_user_sessions,
    revoke_all_user_sessions_tx,
};
pub use authenticate::{
    authenticate, authenticate_bearer, authenticate_verified_bearer, authenticate_with_request,
    try_authenticate_bearer,
};
pub use browser::logout_browser_sessions;
pub use browser_authenticate::authenticate_browser_session;
pub use create::{
    LoginSessionContext, VerifiedPrimaryLogin, create_session_for_principal, login,
    verify_primary_credentials,
};
