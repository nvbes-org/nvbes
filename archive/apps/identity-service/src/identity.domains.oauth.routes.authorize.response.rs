use axum::response::{IntoResponse, Redirect, Response};

use crate::http::error::AppError;

pub(super) fn oauth_success_redirect(
    redirect_uri: &str,
    code: &str,
    state: Option<&str>,
) -> Result<Response, AppError> {
    let mut params = vec![("code", code)];
    if let Some(state) = state {
        params.push(("state", state));
    }
    let redirect_url =
        crate::domains::oauth::hosted_service::build_oauth_redirect_url(redirect_uri, &params)?;
    Ok(Redirect::to(&redirect_url).into_response())
}

pub(super) fn oauth_error_redirect(
    redirect_uri: &str,
    state: Option<&str>,
    error: &AppError,
) -> Result<Response, AppError> {
    let (error_code, description) = oauth_error_response(error);
    let redirect_url = crate::domains::oauth::hosted_service::build_oauth_error_redirect_url(
        redirect_uri,
        error_code,
        description,
        state,
    )?;
    Ok(Redirect::to(&redirect_url).into_response())
}

fn oauth_error_response(error: &AppError) -> (&'static str, &'static str) {
    if error.status.is_server_error() {
        return (
            "server_error",
            "The authorization server could not complete the request.",
        );
    }
    match error.code.as_str() {
        "client_scope_not_allowed" | "invalid_scope" => {
            ("invalid_scope", "The requested scope is not allowed.")
        }
        "client_audience_not_allowed" | "client_resource_not_allowed" | "invalid_target" => (
            "invalid_target",
            "The requested resource server is not allowed.",
        ),
        "access_denied" | "admin_consent_required" => {
            ("access_denied", "The authorization request was denied.")
        }
        _ => ("invalid_request", "The authorization request is invalid."),
    }
}

#[cfg(test)]
mod tests {
    use super::{oauth_error_response, oauth_success_redirect};
    use crate::http::error::AppError;

    #[test]
    fn oauth_redirect_does_not_expose_internal_error_details() {
        let error = AppError::internal("database_error", "secret database topology");

        assert_eq!(
            oauth_error_response(&error),
            (
                "server_error",
                "The authorization server could not complete the request."
            )
        );
    }

    #[test]
    fn audience_policy_failure_maps_to_invalid_target() {
        let error = AppError::forbidden(
            "client_audience_not_allowed",
            "The requested audience is not approved.",
        );

        assert_eq!(
            oauth_error_response(&error),
            (
                "invalid_target",
                "The requested resource server is not allowed."
            )
        );
    }

    #[test]
    fn authorization_success_is_an_http_see_other_redirect() {
        let response = oauth_success_redirect(
            "https://account.example/oauth/callback",
            "gxac_code",
            Some("opaque-state"),
        )
        .expect("redirect should build");

        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("https://account.example/oauth/callback?code=gxac_code&state=opaque-state")
        );
    }
}
