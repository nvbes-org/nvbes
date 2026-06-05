use crate::{app::AppState, http::error::AppError};
use axum::{Form, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/token", post(token))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub device_code: Option<String>,
    pub scope: Option<String>,
    pub audience: Option<String>,
    pub subject_token: Option<String>,
    pub subject_token_type: Option<String>,
    pub actor_token: Option<String>,
    pub actor_token_type: Option<String>,
    pub requested_token_type: Option<String>,
    pub client_assertion_type: Option<String>,
    pub client_assertion: Option<String>,
}

#[utoipa::path(
    post,
    path = "/oauth/token",
    tag = "oauth",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token response", body = crate::domains::oauth::service::TokenView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut client_auth = token_client_auth(
        &headers,
        request.client_id.as_deref(),
        request.client_secret.as_deref(),
        request.client_assertion_type.as_deref(),
        request.client_assertion.as_deref(),
    )?;
    let token_endpoint = format!(
        "{}/oauth/token",
        state.config.api_base_url.trim_end_matches('/')
    );
    client_auth.client_assertion_verified =
        crate::domains::oauth::client_assertion::verify_private_key_jwt(
            &state.db,
            &client_auth,
            &token_endpoint,
        )
        .await?;

    super::enforce_public_oauth_rate_limit_db(
        &state.redis,
        &headers,
        "oauth_token",
        &client_auth.client_id,
        60,
        40,
    )
    .await?;

    let result = match request.grant_type.as_str() {
        "authorization_code" => {
            let code = request.code.ok_or_else(|| {
                AppError::bad_request("missing_code", "The authorization code is required.")
            })?;

            crate::domains::oauth::flows::exchange_code(
                &state,
                crate::domains::oauth::service::ExchangeCodeInput {
                    code,
                    client_id: client_auth.client_id,
                    client_secret: client_auth.client_secret,
                    client_assertion_verified: client_auth.client_assertion_verified,
                    redirect_uri: request.redirect_uri,
                    code_verifier: request.code_verifier,
                },
            )
            .await?
        }
        "client_credentials" => {
            crate::domains::oauth::flows::client_credentials_grant(
                &state,
                client_auth,
                request.scope.as_deref(),
                request.audience.as_deref(),
            )
            .await?
        }
        "refresh_token" => {
            let refresh_token = request.refresh_token.ok_or_else(|| {
                AppError::bad_request("missing_refresh_token", "The refresh token is required.")
            })?;

            crate::domains::oauth::flows::refresh_token(&state, &refresh_token, client_auth).await?
        }
        "urn:ietf:params:oauth:grant-type:device_code" => {
            let device_code = request.device_code.ok_or_else(|| {
                AppError::bad_request("missing_device_code", "The device code is required.")
            })?;

            crate::domains::oauth::device_exchange::exchange_device_code(
                &state,
                crate::domains::oauth::service::ExchangeDeviceCodeInput {
                    client_id: client_auth.client_id,
                    device_code,
                },
            )
            .await?
        }
        "urn:ietf:params:oauth:grant-type:token-exchange" => {
            let subject_token = request.subject_token.ok_or_else(|| {
                AppError::bad_request(
                    "missing_subject_token",
                    "The subject_token parameter is required for token exchange.",
                )
            })?;
            let subject_token_type = request
                .subject_token_type
                .unwrap_or_else(|| "urn:ietf:params:oauth:token-type:access_token".to_string());

            crate::domains::oauth::flows::token_exchange(
                &state,
                crate::domains::oauth::service::TokenExchangeInput {
                    subject_token,
                    subject_token_type,
                    actor_token: request.actor_token,
                    actor_token_type: request.actor_token_type,
                    client_id: client_auth.client_id,
                    client_secret: client_auth.client_secret,
                    client_assertion_verified: client_auth.client_assertion_verified,
                    scope: request.scope,
                    audience: request.audience,
                    resource: None,
                    requested_token_type: request.requested_token_type,
                },
            )
            .await?
        }
        _ => {
            return Err(AppError::bad_request(
                "invalid_grant_type",
                "The requested grant type is not supported.",
            ));
        }
    };

    Ok(Json(serde_json::to_value(result)?))
}

pub(super) fn token_client_auth(
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

    let basic = super::optional_basic_client_auth(headers)?;
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

#[cfg(test)]
mod tests {
    use super::token_client_auth;
    use axum::http::header::AUTHORIZATION;
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    fn basic_headers(client_id: &str, client_secret: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
        );
        headers
    }

    #[test]
    fn token_client_auth_rejects_inconsistent_body_credentials() {
        let headers = basic_headers("client-a", "secret-a");

        let error = token_client_auth(&headers, Some("client-b"), Some("secret-a"), None, None)
            .expect_err("mismatched client_id should fail");

        assert_eq!(error.code, "invalid_client");
    }

    #[test]
    fn token_client_auth_requires_body_client_id_without_basic_auth() {
        let headers = HeaderMap::new();

        let error = token_client_auth(&headers, None, Some("secret-a"), None, None)
            .expect_err("missing client_id should fail");

        assert_eq!(error.code, "missing_client_id");
    }

    #[test]
    fn token_client_auth_accepts_body_credentials_without_basic_auth() {
        let headers = HeaderMap::new();

        let auth = token_client_auth(&headers, Some("client-a"), Some("secret-a"), None, None)
            .expect("body credentials should be accepted");

        assert_eq!(auth.client_id, "client-a");
        assert_eq!(auth.client_secret.as_deref(), Some("secret-a"));
    }

    #[test]
    fn token_client_auth_rejects_multiple_auth_methods() {
        let headers = basic_headers("client-a", "secret-a");

        let error = token_client_auth(
            &headers,
            Some("client-a"),
            None,
            Some(crate::domains::oauth::client_assertion::CLIENT_ASSERTION_TYPE_JWT_BEARER),
            Some("not.a.jwt"),
        )
        .expect_err("mixed auth methods should fail");

        assert_eq!(error.code, "invalid_client");
    }
}
