use axum::http::{HeaderMap, header::AUTHORIZATION, header::USER_AGENT};

use crate::http::error::AppError;

pub fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            AppError::unauthorized("missing_authorization", "Missing Authorization header.")
        })?;

    authorization
        .strip_prefix("Bearer ")
        .map(ToOwned::to_owned)
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| {
            AppError::unauthorized("invalid_authorization", "Invalid Authorization header.")
        })
}

pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    nvbes_core::http::client_ip::client_ip(headers)
}

pub fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub fn country_header<'a>(headers: &'a HeaderMap, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| headers.get(*name).and_then(|value| value.to_str().ok()))
}
