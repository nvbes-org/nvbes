#[path = "identity.oauth.clients.rs"]
pub mod clients;
#[path = "identity.oauth.codes.rs"]
pub mod codes;
#[path = "identity.oauth.error.rs"]
pub mod error;
#[path = "identity.oauth.pkce.rs"]
pub mod pkce;
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
