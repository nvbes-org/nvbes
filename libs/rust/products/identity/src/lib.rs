#[path = "identity.auth.mod.rs"]
pub mod auth;
#[path = "identity.cloud_boundary.rs"]
pub mod cloud_boundary;
#[path = "identity.email.mod.rs"]
pub mod email;
#[path = "identity.error.rs"]
pub mod error;
#[path = "identity.oauth.mod.rs"]
pub mod oauth;

pub use error::{IdentityError, IdentityErrorKind, IdentityResult};
