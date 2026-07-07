use nvbes_core::auth as core;

pub use core::{generate_token, normalize_email, unique_slug, validate_email, validate_password};

#[path = "identity.domains.auth.password.change.rs"]
mod change_impl;
#[path = "identity.domains.auth.password.db.rs"]
pub mod db;
#[path = "identity.domains.auth.password.enterprise.rs"]
pub mod enterprise;
#[path = "identity.domains.auth.password.forgot.rs"]
mod forgot_impl;
#[path = "identity.domains.auth.password.geo.rs"]
mod geo_impl;
#[path = "identity.domains.auth.password.history.rs"]
pub mod history;
#[path = "identity.domains.auth.password.reset.rs"]
mod reset_impl;

#[cfg(test)]
#[path = "identity.domains.auth.password.tests.rs"]
mod tests;

pub use change_impl::change;
pub use enterprise::approve_enterprise_recovery;
pub use forgot_impl::forgot;
pub use reset_impl::reset;

use crate::http::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    Ok(core::hash_password(password)?)
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), AppError> {
    if core::verify_password(password, hash)? {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid_credentials",
            "Invalid email or password.",
        ))
    }
}

pub fn generate_random_token() -> String {
    core::generate_random_token()
}

pub fn token_hash(token: &str) -> String {
    core::token_hash(token)
}

pub fn log_dev_token(token: &str, environment: &str, purpose: &str) {
    core::log_dev_token(token, environment, purpose)
}
