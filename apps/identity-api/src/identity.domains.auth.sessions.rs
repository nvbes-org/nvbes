#[path = "identity.domains.auth.sessions.authenticate.rs"]
mod authenticate;
#[path = "identity.domains.auth.sessions.browser.rs"]
mod browser;
#[path = "identity.domains.auth.sessions.cache.rs"]
pub mod cache;
#[path = "identity.domains.auth.sessions.create.rs"]
mod create;

pub use super::sessions_mgmt::{
    fetch_view, list, logout, revoke, revoke_all_others, revoke_all_user_sessions,
    revoke_all_user_sessions_tx,
};
pub use authenticate::{
    authenticate, authenticate_bearer, authenticate_verified_bearer, try_authenticate_bearer,
};
pub use browser::logout_browser_sessions;
pub use create::{
    LoginSessionContext, VerifiedPrimaryLogin, create_session_for_principal, login,
    verify_primary_credentials,
};
