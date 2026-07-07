use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{app::AppState, http::error::AppError};

use super::hosted_service::{
    StartHostedAuthorizationInput, create_hosted_authorization_state, get_hosted_login_decision,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostedStartRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub request_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostedConsentRequest {
    pub consent_action: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosted-login/start", post(start_hosted_login))
        .route("/hosted-login/{state_id}", get(get_hosted_login))
        .route(
            "/hosted-login/{state_id}/authorize",
            post(authorize_hosted_login),
        )
        .route(
            "/hosted-login/{state_id}/consent",
            post(consent_hosted_login),
        )
}

async fn start_hosted_login(
    State(state): State<AppState>,
    Json(request): Json<HostedStartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hosted = create_hosted_authorization_state(
        &state.redis,
        StartHostedAuthorizationInput {
            client_id: request.client_id,
            redirect_uri: request.redirect_uri,
            scope: request.scope,
            state: request.state,
            request_uri: request.request_uri,
            code_challenge: request.code_challenge,
            code_challenge_method: request.code_challenge_method,
        },
    )
    .await?;

    Ok(Json(serde_json::json!({ "state_id": hosted.state_id })))
}

async fn get_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = get_hosted_login_decision(&state.db, &state.redis, &state_id).await?;
    Ok(Json(serde_json::to_value(decision)?))
}

async fn authorize_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = get_hosted_login_decision(&state.db, &state.redis, &state_id).await?;
    Ok(Json(serde_json::to_value(decision)?))
}

async fn consent_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
    Json(request): Json<HostedConsentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _approved = matches!(request.consent_action.as_deref(), Some("approve"));
    let decision = get_hosted_login_decision(&state.db, &state.redis, &state_id).await?;
    Ok(Json(serde_json::to_value(decision)?))
}
