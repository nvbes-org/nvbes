use crate::domains::oauth::service::ClientAssertionAuthentication;
use crate::http::error::AppError;

use super::ClientAssertionIdentityClaims;

pub fn client_id_from_unverified_assertion(
    body_client_id: Option<&str>,
    assertion: &str,
) -> Result<String, AppError> {
    let token =
        jsonwebtoken::dangerous::insecure_decode::<ClientAssertionIdentityClaims>(assertion)
            .map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion is invalid.")
            })?;
    let client_id = token.claims.iss.trim();
    if client_id.is_empty() || token.claims.sub != token.claims.iss {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion issuer and subject must identify the same client.",
        ));
    }

    if body_client_id.is_some_and(|value| value != client_id) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication credentials are inconsistent.",
        ));
    }

    Ok(client_id.to_string())
}

pub fn assertion_auth(assertion_type: &str, assertion: &str) -> ClientAssertionAuthentication {
    ClientAssertionAuthentication {
        assertion_type: assertion_type.to_string(),
        assertion: assertion.to_string(),
    }
}
