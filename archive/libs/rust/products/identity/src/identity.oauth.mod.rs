#[path = "identity.oauth.secrets.rs"]
pub mod secrets;

pub use secrets::{hash_client_secret, verify_client_secret};
