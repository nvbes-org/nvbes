use crate::domains::auth::types::{StepUpInput, StepUpResult};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, domains::auth::webauthn};
use axum::{
    Json,
    extract::{Extension, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(crate) struct StepUpRequest {
    password: Option<String>,
    totp_code: Option<String>,
    #[schema(value_type = Object)]
    webauthn_response: Option<webauthn_rs::prelude::PublicKeyCredential>,
    webauthn_challenge_id: Option<Uuid>,
    recovery_code: Option<String>,
}

#[utoipa::path(
    post,
    path = "/auth/step-up",
    tag = "auth",
    request_body = StepUpRequest,
    responses(
        (status = 200, description = "Step-up successful", body = StepUpResult),
        (status = 401, description = "Unauthorized or invalid credentials", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn step_up(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<StepUpRequest>,
) -> Result<Json<StepUpResult>, AppError> {
    let webauthn = webauthn::build_webauthn(&state.config)?;
    let result = verification::step_up(
        &state.db,
        &state.redis,
        state.config.auth_step_up_ttl_minutes,
        &webauthn,
        &auth,
        StepUpInput {
            password: request.password,
            totp_code: request.totp_code,
            webauthn_response: request.webauthn_response,
            webauthn_challenge_id: request.webauthn_challenge_id,
            recovery_code: request.recovery_code,
        },
    )
    .await?;

    Ok(Json(result))
}
