use axum::http::HeaderMap;

use crate::http::error::AppError;

pub(crate) fn token_client_auth(
    headers: &HeaderMap,
    body_client_id: Option<&str>,
    body_client_secret: Option<&str>,
    body_client_assertion_type: Option<&str>,
    body_client_assertion: Option<&str>,
) -> Result<crate::domains::oauth::service::ClientAuthentication, AppError> {
    let client_assertion = match (
        body_client_assertion_type
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        body_client_assertion
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) {
        (Some(assertion_type), Some(assertion)) => Some(
            crate::domains::oauth::client_assertion::assertion_auth(assertion_type, assertion),
        ),
        (None, None) => None,
        _ => {
            return Err(AppError::bad_request(
                "invalid_request",
                "client_assertion_type and client_assertion must be provided together.",
            ));
        }
    };

    let basic = super::super::optional_basic_client_auth(headers)?;
    match basic {
        Some((client_id, client_secret)) => {
            if client_assertion.is_some() {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "Only one client authentication method may be used.",
                ));
            }
            if body_client_id.is_some_and(|body_client_id| body_client_id != client_id) {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "Client authentication credentials are inconsistent.",
                ));
            }
            if body_client_secret.is_some_and(|body_secret| body_secret != client_secret) {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "Client authentication credentials are inconsistent.",
                ));
            }
            Ok(crate::domains::oauth::service::ClientAuthentication {
                client_id,
                client_secret: Some(client_secret),
                client_assertion: None,
                client_assertion_verified: false,
            })
        }
        None => {
            if let Some(assertion) = client_assertion {
                if body_client_secret.is_some() {
                    return Err(AppError::unauthorized(
                        "invalid_client",
                        "Only one client authentication method may be used.",
                    ));
                }
                let client_id =
                    crate::domains::oauth::client_assertion::client_id_from_unverified_assertion(
                        body_client_id,
                        &assertion.assertion,
                    )?;
                return Ok(crate::domains::oauth::service::ClientAuthentication {
                    client_id,
                    client_secret: None,
                    client_assertion: Some(assertion),
                    client_assertion_verified: false,
                });
            }

            let client_id = body_client_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::bad_request("missing_client_id", "The client_id is required.")
                })?;
            Ok(crate::domains::oauth::service::ClientAuthentication {
                client_id: client_id.to_string(),
                client_secret: body_client_secret.map(ToOwned::to_owned),
                client_assertion: None,
                client_assertion_verified: false,
            })
        }
    }
}
