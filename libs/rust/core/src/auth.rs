use crate::http::error::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[path = "auth.assurance.rs"]
mod assurance;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Aal {
    #[serde(rename = "aal1")]
    #[default]
    Aal1 = 1,
    #[serde(rename = "aal2")]
    Aal2 = 2,
    #[serde(rename = "aal3")]
    Aal3 = 3,
}

impl Aal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Aal::Aal1 => "aal1",
            Aal::Aal2 => "aal2",
            Aal::Aal3 => "aal3",
        }
    }
}

impl FromStr for Aal {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aal3" => Ok(Aal::Aal3),
            "aal2" => Ok(Aal::Aal2),
            "aal1" | "" => Ok(Aal::Aal1),
            _ => Err(()),
        }
    }
}

pub fn max_aal(left: Aal, right: Aal) -> Aal {
    if left >= right { left } else { right }
}

pub fn step_up_required_error() -> AppError {
    AppError::unauthorized("step_up_required", "Please verify again before continuing.")
}

pub fn workspace_switch_step_up_required_error() -> AppError {
    AppError::unauthorized(
        "step_up_required",
        "Please verify again before switching workspaces.",
    )
}

#[path = "auth.helpers.birthdate.rs"]
mod birthdate;
#[path = "auth.helpers.password.rs"]
mod password;
#[cfg(test)]
#[path = "auth.helpers.tests.rs"]
mod tests;
#[path = "auth.helpers.tokens.rs"]
mod tokens;
#[path = "auth.helpers.validation.rs"]
mod validation;

pub use assurance::{
    PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS, has_recent_phishing_resistant_authentication,
    is_phishing_resistant_method,
};
pub use birthdate::{parse_birthdate, today_in_region, validate_birthdate};
pub use password::{
    dummy_verify_password, hash_password, hash_password_with_pepper, verify_and_check_rehash,
    verify_password, verify_password_with_pepper,
};
pub use tokens::{
    generate_random_token, generate_token, random_challenge, token_hash, token_hash_b64,
    unique_slug,
};
pub use validation::{
    MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH, normalize_email, require_non_empty, slugify,
    validate_email, validate_password,
};
