use crate::http::error::AppError;

use super::ParRequest;

pub(super) fn validate_par_request(request: &ParRequest) -> Result<(), AppError> {
    let redirect_uri = request
        .redirect_uri
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();

    if redirect_uri.is_empty() {
        return Err(AppError::bad_request(
            "missing_redirect_uri",
            "The redirect_uri parameter is required for PAR.",
        ));
    }

    if request
        .state
        .as_deref()
        .is_none_or(|state| state.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "state_required",
            "The state parameter is required for authorization requests.",
        ));
    }

    let requests_openid = request
        .scope
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .any(|scope| scope == "openid");
    if requests_openid
        && request
            .nonce
            .as_deref()
            .is_none_or(|nonce| nonce.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "nonce_required",
            "The nonce parameter is required when the openid scope is requested.",
        ));
    }
    if request
        .scope
        .as_deref()
        .is_some_and(|scope| scope.len() > 2048 || scope.split_whitespace().count() > 64)
    {
        return Err(AppError::bad_request(
            "invalid_scope",
            "At most 64 scopes and 2048 scope characters are allowed.",
        ));
    }
    if request
        .resource
        .as_ref()
        .is_some_and(|resources| resources.len() > 8)
    {
        return Err(AppError::bad_request(
            "invalid_target",
            "At most eight resource indicators are allowed.",
        ));
    }
    crate::domains::oauth::validation::resolve_access_token_audience(
        request.audience.as_deref(),
        request.resource.as_deref().unwrap_or_default(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ParRequest, validate_par_request};

    #[test]
    fn requires_state_for_pushed_authorization_requests() {
        let request = ParRequest {
            response_type: Some("code".to_string()),
            client_id: Some("client".to_string()),
            client_secret: None,
            redirect_uri: Some("https://client.example/callback".to_string()),
            scope: Some("openid".to_string()),
            state: None,
            nonce: None,
            audience: None,
            resource: None,
            authorization_details: None,
            code_challenge: Some("challenge".to_string()),
            code_challenge_method: Some("S256".to_string()),
            client_assertion_type: None,
            client_assertion: None,
            request: None,
        };

        let error = validate_par_request(&request).expect_err("state should be required");
        assert_eq!(error.code, "state_required");
    }

    #[test]
    fn requires_one_explicit_resource_server_target() {
        let request = ParRequest {
            response_type: Some("code".to_string()),
            client_id: Some("client".to_string()),
            client_secret: None,
            redirect_uri: Some("https://client.example/callback".to_string()),
            scope: Some("account.profile.read".to_string()),
            state: Some("opaque-state".to_string()),
            nonce: None,
            audience: None,
            resource: None,
            authorization_details: None,
            code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string()),
            code_challenge_method: Some("S256".to_string()),
            client_assertion_type: None,
            client_assertion: None,
            request: None,
        };

        let error =
            validate_par_request(&request).expect_err("resource server target should be required");
        assert_eq!(error.code, "invalid_target");
    }
}
