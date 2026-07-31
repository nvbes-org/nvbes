use crate::http::error::AppError;

use super::models::OAuthClientKeyPurpose;

pub(super) fn validate_public_jwk(
    purpose: OAuthClientKeyPurpose,
    jwk: &serde_json::Value,
    high_assurance: bool,
) -> Result<(), AppError> {
    key_id(jwk)?;
    for private_parameter in ["d", "p", "q", "dp", "dq", "qi", "oth", "k"] {
        if jwk.get(private_parameter).is_some() {
            return Err(invalid_key("Only public JWK parameters may be registered."));
        }
    }
    match purpose {
        OAuthClientKeyPurpose::ClientAuthentication => {
            let valid = if high_assurance {
                crate::domains::oauth::client_assertion::is_high_assurance_client_assertion_jwk(jwk)
            } else {
                crate::domains::oauth::client_assertion::is_supported_client_assertion_public_jwk(
                    jwk,
                )
            };
            if !valid {
                return Err(invalid_key(
                    "The client authentication JWK is unsupported for this security profile.",
                ));
            }
        }
        OAuthClientKeyPurpose::RequestObject => {
            crate::domains::oauth::jar::validate_high_assurance_jwks(
                &serde_json::json!({ "keys": [jwk] }),
            )?;
        }
    }
    Ok(())
}

pub(super) fn key_id(jwk: &serde_json::Value) -> Result<&str, AppError> {
    jwk.get("kid")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|kid| !kid.is_empty() && kid.len() <= 255)
        .ok_or_else(|| invalid_key("Every client JWK requires a kid of at most 255 bytes."))
}

pub(super) fn invalid_key(message: &'static str) -> AppError {
    AppError::bad_request("invalid_oauth_client_key", message)
}
