use crate::app::AppState;
use crate::domains::auth::mfa::totp as mfa_totp;
use crate::domains::auth::routes::mfa::types::{TotpConfirmRequest, TotpSetupRequest};
use crate::domains::auth::types::{
    TotpConfirmInput, TotpConfirmResult, TotpSetupInput, TotpSetupResult,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::{AuthContext, jwt_auth_middleware};
use axum::{
    Json, Router,
    extract::{Extension, State},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

pub fn router(state: &AppState) -> Router<AppState> {
    let auth_middleware = jwt_auth_middleware;

    Router::new()
        .route(
            "/totp/setup",
            post(begin_totp_enrollment).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/totp/confirm",
            post(confirm_totp_enrollment).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
}

#[utoipa::path(
    post,
    path = "/auth/mfa/totp/setup",
    tag = "auth",
    request_body = TotpSetupRequest,
    responses(
        (status = 200, description = "TOTP enrollment started", body = TotpSetupResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn begin_totp_enrollment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<TotpSetupRequest>,
) -> Result<Json<TotpSetupResult>, AppError> {
    crate::domains::auth::verification::require_recent_step_up(
        &state.redis,
        &auth,
        Some(nvbes_core::auth::Aal::Aal2),
    )
    .await?;
    let result = mfa_totp::begin_totp(
        &state.db,
        auth.user_id,
        TotpSetupInput {
            label: request.label,
        },
    )
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/auth/mfa/totp/confirm",
    tag = "auth",
    request_body = TotpConfirmRequest,
    responses(
        (status = 200, description = "TOTP enrollment confirmed", body = TotpConfirmResult),
        (status = 400, description = "Invalid TOTP code", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn confirm_totp_enrollment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<TotpConfirmRequest>,
) -> Result<Json<TotpConfirmResult>, AppError> {
    let result = mfa_totp::confirm_totp(
        &state.db,
        auth.user_id,
        TotpConfirmInput {
            factor_id: request.factor_id,
            code: request.code,
        },
    )
    .await?;

    Ok(Json(result))
}
