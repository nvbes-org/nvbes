#[path = "identity.domains.oauth.validation.normalize.rs"]
mod normalize;
#[path = "identity.domains.oauth.validation.parse.rs"]
mod parse;
#[path = "identity.domains.oauth.validation.pkce.rs"]
mod pkce;
#[path = "identity.domains.oauth.validation.redirects.rs"]
mod redirects;
#[path = "identity.domains.oauth.validation.target.rs"]
mod target;
#[cfg(test)]
#[path = "identity.domains.oauth.validation.tests.rs"]
mod tests;

pub use normalize::{normalize_resources, normalize_scopes};
pub use parse::{parse_client_policy_status, parse_client_type, parse_step_up_level};
pub use pkce::{is_public_client_type, validate_pkce_for_authorize, validate_pkce_for_exchange};
pub use redirects::{validate_redirect_uri_allowed, validate_redirect_uri_match};
pub use target::resolve_access_token_audience;
