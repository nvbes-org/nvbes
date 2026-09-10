//! Contracts and protocol capabilities of the active Identity service.

#[path = "identity.oauth.mod.rs"]
pub mod oauth;

#[path = "identity.auth.rs"]
pub mod auth;
#[path = "identity.browser.rs"]
pub mod browser;
#[path = "identity.mfa.rs"]
pub mod mfa;
#[path = "identity.mfa.crypto.rs"]
pub mod mfa_crypto;
#[path = "identity.mfa.recovery.rs"]
pub mod mfa_recovery;
#[path = "identity.notifications.command.rs"]
pub mod notification_command;
#[path = "identity.notifications.queue.rs"]
mod notification_queue;
#[path = "identity.notifications.dispatch.rs"]
pub mod notification_dispatch;
#[path = "identity.rate_limits.rs"]
pub mod rate_limits;
#[path = "identity.sessions.lock.rs"]
mod session_locks;
#[path = "identity.totp.rs"]
pub mod totp;
#[path = "identity.webauthn.rs"]
pub mod webauthn;

#[path = "identity.authentication.rs"]
mod authentication;
#[path = "identity.authentication.enrollment.rs"]
mod enrollment_policy;
#[path = "identity.authentication.management.rs"]
mod factor_management;
#[path = "identity.tokens.rs"]
pub mod tokens;
#[path = "identity.tokens.claims.rs"]
pub mod tokens_claims;
#[path = "identity.tokens.config.rs"]
pub mod tokens_config;
#[path = "identity.tokens.error.rs"]
pub mod tokens_error;
#[path = "identity.tokens.grants.rs"]
mod tokens_grants;
#[path = "identity.tokens.keys.rs"]
mod tokens_keys;
#[path = "identity.tokens.policy.rs"]
mod tokens_policy;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.test.fixtures.rs"]
mod test_fixtures;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.browser.e2e_fixture.rs"]
mod browser_e2e_fixture;
