#[path = "account.auth.mod.rs"]
pub mod auth;
#[path = "account.cloud_boundary.rs"]
pub mod cloud_boundary;
#[path = "account.email.mod.rs"]
pub mod email;
#[path = "account.error.rs"]
pub mod error;
#[path = "account.oauth.mod.rs"]
pub mod oauth;

pub use error::{AccountError, AccountResult};
