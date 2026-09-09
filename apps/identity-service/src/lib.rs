//! Contracts and protocol capabilities of the active Identity service.

#[path = "identity.oauth.mod.rs"]
pub mod oauth;

#[path = "identity.auth.rs"]
pub mod auth;
#[path = "identity.browser.rs"]
pub mod browser;
#[path = "identity.rate_limits.rs"]
pub mod rate_limits;

#[path = "identity.authentication.rs"]
mod authentication;
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
