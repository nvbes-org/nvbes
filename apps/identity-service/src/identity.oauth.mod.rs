#[path = "identity.oauth.clients.rs"]
pub mod clients;
#[path = "identity.oauth.codes.rs"]
pub mod codes;
#[path = "identity.oauth.consent.rs"]
pub mod consent;
#[path = "identity.oauth.dpop.rs"]
pub mod dpop;
#[path = "identity.oauth.error.rs"]
pub mod error;
#[path = "identity.oauth.http.rs"]
pub mod http;
#[path = "identity.oauth.interactions.rs"]
pub mod interactions;
#[path = "identity.oauth.limits.rs"]
mod limits;
#[path = "identity.oauth.login.rs"]
mod login;
#[path = "identity.oauth.maintenance.rs"]
pub mod maintenance;
#[path = "identity.oauth.metadata.rs"]
pub mod metadata;
#[path = "identity.oauth.passkey_login.rs"]
pub mod passkey_login;
#[path = "identity.oauth.pkce.rs"]
pub mod pkce;
#[path = "identity.oauth.session.rs"]
mod session;
#[path = "identity.oauth.store.rs"]
pub mod store;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.database.tests.rs"]
mod database_tests;
#[path = "identity.oauth.request.rs"]
pub mod request;

#[cfg(test)]
#[path = "identity.oauth.validation.tests.rs"]
mod validation_tests;
