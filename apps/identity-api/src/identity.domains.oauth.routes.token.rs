use crate::{app::AppState, http::error::AppError};
use axum::{Form, Json, Router, extract::State, http::HeaderMap, routing::post};
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
        )
        .await?;

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

    let result = grants::handle_token_grant(&state, request, client_auth).await?;

    Ok(Json(serde_json::to_value(result)?))
}
