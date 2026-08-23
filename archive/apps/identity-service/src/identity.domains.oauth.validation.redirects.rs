use crate::http::error::AppError;

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
