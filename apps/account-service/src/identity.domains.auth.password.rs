use nvbes_core::auth as core;

pub use core::{generate_token, normalize_email, unique_slug, validate_email, validate_password};

#[path = "identity.domains.auth.password.change.rs"]
mod change_impl;
#[path = "identity.domains.auth.password.db.rs"]
pub mod db;
#[path = "identity.domains.auth.password.forgot.rs"]
mod forgot_impl;
#[path = "identity.domains.auth.password.geo.rs"]
mod geo_impl;
#[path = "identity.domains.auth.password.history.rs"]
pub mod history;
#[path = "identity.domains.auth.password.reset.rs"]
mod reset_impl;
#[path = "identity.domains.auth.password.review.rs"]
pub mod review;

#[cfg(test)]
#[path = "identity.domains.auth.password.tests.rs"]
mod tests;

pub use change_impl::change;
pub use forgot_impl::forgot;
pub use reset_impl::reset;

use crate::http::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash_password_with_pepper(password, None)
}

pub fn hash_password_with_pepper(password: &str, pepper: Option<&str>) -> Result<String, AppError> {
    Ok(core::hash_password_with_pepper(
        password,
        pepper.map(|p| p.as_bytes()),
    )?)
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), AppError> {
    verify_password_with_pepper(hash, password, None)
}

pub fn verify_password_with_pepper(
    hash: &str,
    password: &str,
    pepper: Option<&str>,
) -> Result<(), AppError> {
    let (valid, _) = verify_and_check_rehash(hash, password, pepper)?;
    if valid {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid_credentials",
            "Invalid email or password.",
        ))
    }
}

pub fn verify_and_check_rehash(
    hash: &str,
    password: &str,
    pepper: Option<&str>,
) -> Result<(bool, bool), AppError> {
    Ok(core::verify_and_check_rehash(
        password,
        hash,
        pepper.map(|p| p.as_bytes()),
    )?)
}

pub fn dummy_verify_password(password: &str, pepper: Option<&str>) {
    core::dummy_verify_password(password, pepper.map(|p| p.as_bytes()));
}

pub fn generate_random_token() -> String {
    core::generate_random_token()
}

pub fn token_hash(token: &str) -> String {
    core::token_hash(token)
}
