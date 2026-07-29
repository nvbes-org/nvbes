use crate::{app::AppState, http::error::AppError};

use super::TokenRequest;

pub(super) async fn handle_token_grant(
    state: &AppState,
    request: TokenRequest,
    client_auth: crate::domains::oauth::service::ClientAuthentication,
    security_profile: crate::domains::oauth::profiles::OAuthSecurityProfile,
    token_confirmation: Option<crate::domains::auth::jwt::TokenConfirmation>,
) -> Result<crate::domains::oauth::service::TokenView, AppError> {
    if security_profile == crate::domains::oauth::profiles::OAuthSecurityProfile::HighAssurance
        && !matches!(
            request.grant_type.as_str(),
            "authorization_code" | "refresh_token" | "client_credentials"
        )
    {
        return Err(AppError::bad_request(
            "unsupported_high_assurance_grant",
            "This grant type is not available to high-assurance OAuth clients.",
        ));
    }
    match request.grant_type.as_str() {
        "authorization_code" => {
            let code = request.code.ok_or_else(|| {
                AppError::bad_request("missing_code", "The authorization code is required.")
            })?;

            crate::domains::oauth::flows::exchange_code(
                &state.db,
                &state.redis,
                &state.jwt,
                state.config.auth_refresh_token_ttl_hours,
                crate::domains::oauth::service::ExchangeCodeInput {
                    code,
                    client_id: client_auth.client_id,
                    client_secret: client_auth.client_secret,
                    client_assertion_verified: client_auth.client_assertion_verified,
                    redirect_uri: request.redirect_uri,
                    code_verifier: request.code_verifier,
                    token_confirmation,
                },
            )
            .await
        }
        "client_credentials" => {
            crate::domains::oauth::flows::client_credentials_grant(
                &state.db,
                &state.jwt,
                client_auth,
                request.scope.as_deref(),
                request.audience.as_deref(),
                token_confirmation,
            )
            .await
        }
        "refresh_token" => {
            let refresh_token = request.refresh_token.ok_or_else(|| {
                AppError::bad_request("missing_refresh_token", "The refresh token is required.")
            })?;

            crate::domains::oauth::flows::refresh_token(
                &state.db,
                &state.redis,
                &state.jwt,
                state.config.auth_refresh_token_ttl_hours,
                &refresh_token,
                client_auth,
                security_profile,
                token_confirmation,
            )
            .await
        }
        "urn:ietf:params:oauth:grant-type:device_code" => {
            let device_code = request.device_code.ok_or_else(|| {
                AppError::bad_request("missing_device_code", "The device code is required.")
            })?;

            crate::domains::oauth::device_exchange::exchange_device_code(
                &state.db,
                &state.redis,
                &state.jwt,
                state.config.auth_refresh_token_ttl_hours,
                crate::domains::oauth::service::ExchangeDeviceCodeInput {
                    client_id: client_auth.client_id,
                    device_code,
                },
                token_confirmation,
            )
            .await
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
                &state.db,
                &state.redis,
                &state.jwt,
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
                token_confirmation,
            )
            .await
        }
        _ => Err(AppError::bad_request(
            "invalid_grant_type",
            "The requested grant type is not supported.",
        )),
    }
}
