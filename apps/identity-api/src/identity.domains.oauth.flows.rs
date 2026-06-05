#[path = "identity.domains.oauth.flows.client_credentials.rs"]
pub mod client_credentials;
#[path = "identity.domains.oauth.flows.codes.rs"]
pub mod codes;
#[path = "identity.domains.oauth.flows.refresh.rs"]
pub mod refresh;
#[path = "identity.domains.oauth.flows.token_exchange.rs"]
pub mod token_exchange;
#[path = "identity.domains.oauth.flows.tokens.rs"]
pub mod tokens;

pub use super::service::types::*;

pub use client_credentials::client_credentials_grant;
pub use codes::{create_authorization_code, exchange_code};
pub use refresh::{refresh_token, revoke_refresh_family};
pub use token_exchange::token_exchange;
pub use tokens::{generate_tokens, introspect_token};
