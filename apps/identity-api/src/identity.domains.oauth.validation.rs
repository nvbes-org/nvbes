use crate::http::error::AppError;
use base64::Engine;
use sha2::{Digest, Sha256};

pub fn normalize_scopes(mut scopes: Vec<String>) -> Vec<String> {
    scopes = scopes
        .into_iter()
        .map(|scope| scope.trim().to_lowercase())
        .filter(|scope| !scope.is_empty())
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

pub fn normalize_resources(mut values: Vec<String>) -> Vec<String> {
    values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
}

pub fn is_public_client_type(client_type: &str) -> bool {
    matches!(
        client_type,
        "public" | "native" | "desktop" | "device" | "mobile" | "iot"
    )
}

pub fn validate_pkce_for_authorize(
    client_type: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
) -> Result<(), AppError> {
    if is_public_client_type(client_type) {
        let challenge = code_challenge.ok_or_else(|| {
            AppError::bad_request(
                "pkce_required",
                "PKCE code_challenge is required for public/native clients.",
            )
        })?;
        if challenge.trim().is_empty() {
            return Err(AppError::bad_request(
                "pkce_required",
                "PKCE code_challenge is required for public/native clients.",
            ));
        }
    }
    if code_challenge.is_some() {
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
    }
    Ok(())
}

pub fn validate_pkce_for_exchange(
    client_type: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
    code_verifier: Option<&str>,
) -> Result<(), AppError> {
    if code_challenge.is_none() {
        if is_public_client_type(client_type) {
            return Err(AppError::bad_request(
                "pkce_required",
                "PKCE verifier is required for public/native clients.",
            ));
        }
        return Ok(());
    }
    let verifier = code_verifier.ok_or_else(|| {
        AppError::bad_request(
            "pkce_required",
            "PKCE verifier is required for this authorization code.",
        )
    })?;
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
    };

    let digest = Sha256::digest(verifier.as_bytes());
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    let valid = encoded == challenge;
    if !valid {
        return Err(AppError::bad_request(
            "invalid_grant",
            "Invalid PKCE verifier.",
        ));
    }
    Ok(())
}

pub fn validate_redirect_uri_allowed(
    allowed_redirect_uris: &[String],
    redirect_uri: &str,
) -> Result<(), AppError> {
    if allowed_redirect_uris.iter().any(|uri| uri == redirect_uri) {
        return Ok(());
    }

    Err(AppError::bad_request(
        "invalid_redirect_uri",
        "The redirect URI is not registered for this OAuth client.",
    ))
}

pub fn validate_redirect_uri_match(
    expected_redirect_uri: &str,
    provided_redirect_uri: Option<&str>,
) -> Result<(), AppError> {
    let provided_redirect_uri = provided_redirect_uri.ok_or_else(|| {
        AppError::bad_request(
            "invalid_request",
            "The redirect URI is required for this authorization code.",
        )
    })?;

    if provided_redirect_uri == expected_redirect_uri {
        Ok(())
    } else {
        Err(AppError::bad_request(
            "invalid_grant",
            "The redirect URI does not match the authorization request.",
        ))
    }
}

pub fn parse_step_up_level(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "aal1" | "aal2" | "aal3" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported step-up level.",
        )),
    }
}

pub fn parse_client_policy_status(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "active" | "restricted" | "blocked" | "pending_approval" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported client policy status.",
        )),
    }
}

pub fn parse_client_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "confidential" | "public" | "native" | "desktop" | "device" | "mobile" | "iot"
        | "service" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported client type.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use sha2::Digest;

    #[test]
    fn normalize_scopes_lowercases_sorts_and_deduplicates() {
        let scopes = normalize_scopes(vec![
            " email ".to_string(),
            "openid".to_string(),
            "profile".to_string(),
            "Email".to_string(),
            "".to_string(),
        ]);

        assert_eq!(scopes, vec!["email", "openid", "profile"]);
    }

    #[test]
    fn normalize_resources_trims_sorts_and_deduplicates() {
        let resources = normalize_resources(vec![
            " https://api.example.com ".to_string(),
            "urn:api:1".to_string(),
            "urn:api:1".to_string(),
        ]);

        assert_eq!(resources, vec!["https://api.example.com", "urn:api:1"]);
    }

    #[test]
    fn validate_pkce_for_authorize_requires_challenge_for_public_clients() {
        let error =
            validate_pkce_for_authorize("public", None, Some("S256")).expect_err("pkce required");

        assert_eq!(error.code, "pkce_required");
    }

    #[test]
    fn validate_pkce_for_exchange_accepts_s256_verifier() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

        assert!(
            validate_pkce_for_exchange(
                "confidential",
                Some(&challenge),
                Some("S256"),
                Some(verifier),
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_pkce_for_authorize_rejects_plain_method() {
        let error = validate_pkce_for_authorize("public", Some("challenge"), Some("plain"))
            .expect_err("plain PKCE should fail");

        assert_eq!(error.code, "pkce_s256_required");
    }

    #[test]
    fn validate_pkce_for_exchange_requires_explicit_s256_method() {
        let error = validate_pkce_for_exchange("public", Some("challenge"), None, Some("verifier"))
            .expect_err("missing PKCE method should fail");

        assert_eq!(error.code, "pkce_s256_required");
    }

    #[test]
    fn parse_step_up_level_accepts_supported_values() {
        assert_eq!(parse_step_up_level(" AAL2 ").unwrap(), "aal2");
        assert_eq!(parse_client_type(" Service ").unwrap(), "service");
        assert_eq!(
            parse_client_policy_status(" pending_approval ").unwrap(),
            "pending_approval"
        );
    }

    #[test]
    fn parse_step_up_level_rejects_unknown_values() {
        let error = parse_step_up_level("aal4").expect_err("invalid aal should fail");

        assert_eq!(error.code, "validation_failed");
    }

    #[test]
    fn validate_redirect_uri_match_requires_provided_uri() {
        let error = validate_redirect_uri_match("https://example.com/callback", None)
            .expect_err("missing uri");

        assert_eq!(error.code, "invalid_request");
    }

    #[test]
    fn validate_redirect_uri_allowed_accepts_registered_uri() {
        let allowed = vec![
            "https://example.com/callback".to_string(),
            "https://example.com/alt".to_string(),
        ];

        assert!(validate_redirect_uri_allowed(&allowed, "https://example.com/alt").is_ok());
    }

    #[test]
    fn validate_redirect_uri_allowed_rejects_unregistered_uri() {
        let allowed = vec!["https://example.com/callback".to_string()];
        let err = validate_redirect_uri_allowed(&allowed, "https://attacker.example/callback")
            .expect_err("expected redirect URI validation to fail");

        assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "invalid_redirect_uri");
        assert_eq!(
            err.message,
            "The redirect URI is not registered for this OAuth client."
        );
    }

    #[test]
    fn validate_redirect_uri_match_rejects_mismatch() {
        let err = validate_redirect_uri_match(
            "https://example.com/callback",
            Some("https://example.com/other"),
        )
        .expect_err("expected redirect URI mismatch to fail");

        assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "invalid_grant");
        assert_eq!(
            err.message,
            "The redirect URI does not match the authorization request."
        );
    }
}
