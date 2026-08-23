use crate::http::error::AppError;
use base64::Engine;
use sha2::{Digest, Sha256};

pub fn is_public_client_type(client_type: &str) -> bool {
    matches!(
        client_type,
        "public" | "native" | "desktop" | "device" | "mobile" | "iot"
    )
}

pub fn validate_pkce_for_authorize(
    _client_type: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
) -> Result<(), AppError> {
    let challenge = code_challenge.ok_or_else(|| {
        AppError::bad_request(
            "pkce_required",
            "PKCE code_challenge is required for every authorization-code client.",
        )
    })?;
    if challenge.trim().is_empty() {
        return Err(AppError::bad_request(
            "pkce_required",
            "PKCE code_challenge is required for every authorization-code client.",
        ));
    }
    if challenge.len() != 43
        || !challenge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AppError::bad_request(
            "invalid_code_challenge",
            "The S256 code_challenge must be a 43-character base64url value.",
        ));
    }

    let method = code_challenge_method
        .map(str::trim)
        .filter(|method| !method.is_empty())
        .ok_or_else(|| {
            AppError::bad_request(
                "pkce_s256_required",
                "PKCE code_challenge_method=S256 is required.",
            )
        })?;

    if !method.eq_ignore_ascii_case("S256") {
        return Err(AppError::bad_request(
            "pkce_s256_required",
            "Only PKCE code_challenge_method=S256 is supported.",
        ));
    }

    Ok(())
}

pub fn validate_pkce_for_exchange(
    _client_type: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
    code_verifier: Option<&str>,
) -> Result<(), AppError> {
    if code_challenge.is_none() {
        return Err(AppError::bad_request(
            "pkce_required",
            "PKCE is required for every authorization-code client.",
        ));
    }

    let verifier = code_verifier.ok_or_else(|| {
        AppError::bad_request(
            "pkce_required",
            "PKCE verifier is required for this authorization code.",
        )
    })?;
    if !(43..=128).contains(&verifier.len())
        || !verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
    {
        return Err(AppError::bad_request(
            "invalid_grant",
            "The PKCE code_verifier is invalid.",
        ));
    }
    let challenge = code_challenge.unwrap_or_default();
    let method = code_challenge_method
        .map(str::trim)
        .filter(|method| !method.is_empty())
        .ok_or_else(|| {
            AppError::bad_request(
                "pkce_s256_required",
                "PKCE code_challenge_method=S256 is required.",
            )
        })?;

    if !method.eq_ignore_ascii_case("S256") {
        return Err(AppError::bad_request(
            "pkce_s256_required",
            "Only PKCE code_challenge_method=S256 is supported.",
        ));
    }

    let digest = Sha256::digest(verifier.as_bytes());
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    if encoded != challenge {
        return Err(AppError::bad_request(
            "invalid_grant",
            "Invalid PKCE verifier.",
        ));
    }

    Ok(())
}
