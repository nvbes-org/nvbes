use crate::{app::AppState, http::error::AppError};
use axum::{Extension, Form, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use std::time::Duration;

#[path = "identity.domains.oauth.routes.token.auth.rs"]
mod auth;
#[path = "identity.domains.oauth.routes.token.grants.rs"]
mod grants;
#[cfg(test)]
#[path = "identity.domains.oauth.routes.token.tests.rs"]
mod tests;

pub(super) use auth::token_client_auth;

pub fn router() -> Router<AppState> {
    Router::new().route("/token", post(token))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
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
    dpop: Option<Extension<crate::http::middleware::dpop::DpopContext>>,
    mtls: Option<Extension<crate::http::mtls::MtlsCertificateThumbprint>>,
    Form(request): Form<TokenRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut client_auth = auth::token_client_auth(
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
            state.config.api_base_url.trim_end_matches('/'),
        )
        .await?;
    let client_security =
        crate::domains::oauth::profiles::load_client_security(&state.db, &client_auth.client_id)
            .await?;
    let token_confirmation = enforce_token_endpoint_security(
        &state,
        &client_auth,
        &client_security,
        dpop.as_ref().map(|Extension(context)| context),
        mtls.as_ref().map(|Extension(thumbprint)| thumbprint),
    )?;

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_token",
        &client_auth.client_id,
        60,
        40,
        Duration::from_secs(60),
    )
    .await?;

    let result = grants::handle_token_grant(
        &state,
        request,
        client_auth,
        client_security.profile,
        token_confirmation,
    )
    .await?;

    Ok(Json(serde_json::to_value(result)?))
}

pub(super) fn enforce_token_endpoint_security(
    state: &AppState,
    client_auth: &crate::domains::oauth::service::ClientAuthentication,
    security: &crate::domains::oauth::profiles::OAuthClientSecurity,
    dpop: Option<&crate::http::middleware::dpop::DpopContext>,
    mtls: Option<&crate::http::mtls::MtlsCertificateThumbprint>,
) -> Result<Option<crate::domains::auth::jwt::TokenConfirmation>, AppError> {
    use crate::domains::oauth::profiles::{OAuthSecurityProfile, OAuthSenderConstraint};

    if security.profile == OAuthSecurityProfile::HighAssurance
        && !client_auth.client_assertion_verified
    {
        return Err(AppError::unauthorized(
            "invalid_client",
            "High-assurance clients must authenticate with private_key_jwt.",
        ));
    }
    if dpop.is_some() && mtls.is_some() {
        return Err(AppError::bad_request(
            "multiple_sender_constraints",
            "Use exactly one sender-constraining mechanism per token request.",
        ));
    }

    match security.sender_constraint {
        Some(OAuthSenderConstraint::Dpop) => {
            if security.profile == OAuthSecurityProfile::HighAssurance && state.dpop_nonce.is_none()
            {
                return Err(AppError::internal(
                    "dpop_nonce_configuration_required",
                    "High-assurance DPoP clients require the server nonce and replay store.",
                ));
            }
            let dpop = dpop.ok_or_else(|| {
                AppError::unauthorized(
                    "dpop_required",
                    "This OAuth client requires a valid DPoP proof.",
                )
            })?;
            Ok(Some(crate::domains::auth::jwt::TokenConfirmation::dpop(
                dpop.jkt.clone(),
            )))
        }
        Some(OAuthSenderConstraint::Mtls) => {
            let actual = mtls.ok_or_else(|| {
                AppError::unauthorized(
                    "mtls_required",
                    "This OAuth client must use the mutual-TLS token endpoint.",
                )
            })?;
            let registered = security
                .tls_client_certificate_sha256
                .as_deref()
                .ok_or_else(|| AppError::unauthorized("invalid_client", "mTLS key is missing."))?;
            if actual.0 != registered {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "The mutual-TLS certificate is not registered for this OAuth client.",
                ));
            }
            Ok(Some(crate::domains::auth::jwt::TokenConfirmation::mtls(
                actual.0.clone(),
            )))
        }
        None => Ok(dpop.map(|context| {
            crate::domains::auth::jwt::TokenConfirmation::dpop(context.jkt.clone())
        })),
    }
}
