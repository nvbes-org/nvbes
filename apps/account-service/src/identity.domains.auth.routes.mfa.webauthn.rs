use crate::app::AppState;
use crate::domains::auth::routes::mfa::types::{
    WebauthnRegisterFinishRequest, WebauthnRegisterStartRequest,
};
use crate::domains::auth::types::{
    TotpConfirmResult, WebauthnAuthStartResult, WebauthnRegisterStartResult,
};
use crate::domains::auth::webauthn as webauthn_mod;
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
            "/webauthn/start",
            post(webauthn_auth_start).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/webauthn/register/start",
            post(begin_webauthn_enrollment).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/webauthn/register/finish",
            post(confirm_webauthn_enrollment).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
}

#[utoipa::path(
    post,
    path = "/auth/mfa/webauthn/start",
    tag = "auth",
    responses(
        (status = 200, description = "WebAuthn authentication challenge", body = WebauthnAuthStartResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn webauthn_auth_start(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<WebauthnAuthStartResult>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::internal(
            "tenant_context_missing",
            "Authenticated session is missing tenant context.",
        )
    })?;
    let webauthn = webauthn_mod::build_webauthn(&state.config)?;
    let (challenge_id, options) = webauthn_mod::start_authentication(
        &state.db,
        &state.redis,
        &webauthn,
        auth.session_id,
        auth.user_id,
        tenant_id,
        auth.workspace_id,
    )
    .await?;
    Ok(Json(WebauthnAuthStartResult {
        challenge_id,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/mfa/webauthn/register/start",
    tag = "auth",
    request_body = WebauthnRegisterStartRequest,
    responses(
        (status = 200, description = "WebAuthn registration challenge", body = WebauthnRegisterStartResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn begin_webauthn_enrollment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<WebauthnRegisterStartRequest>,
) -> Result<Json<WebauthnRegisterStartResult>, AppError> {
    crate::domains::auth::verification::require_recent_step_up(
        &state.redis,
        &auth,
        Some(nvbes_core::auth::Aal::Aal2),
    )
    .await?;
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::internal(
            "tenant_context_missing",
            "Authenticated session is missing tenant context.",
        )
    })?;
    let webauthn = webauthn_mod::build_webauthn(&state.config)?;
    let (challenge_id, options) = webauthn_mod::start_registration(
        &state.db,
        &state.redis,
        &webauthn,
        auth.session_id,
        auth.user_id,
        &auth.display_name,
        tenant_id,
        auth.workspace_id,
        request.label,
        request.kind,
    )
    .await?;
    Ok(Json(WebauthnRegisterStartResult {
        factor_id: challenge_id,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/mfa/webauthn/register/finish",
    tag = "auth",
    request_body = WebauthnRegisterFinishRequest,
    responses(
        (status = 200, description = "WebAuthn enrollment confirmed", body = TotpConfirmResult),
        (status = 400, description = "Invalid registration response", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn confirm_webauthn_enrollment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<WebauthnRegisterFinishRequest>,
) -> Result<Json<TotpConfirmResult>, AppError> {
    let webauthn = webauthn_mod::build_webauthn(&state.config)?;
    webauthn_mod::finish_registration(
        &state.db,
        &state.redis,
        &webauthn,
        auth.session_id,
        auth.user_id,
        request.factor_id,
        request.reg,
    )
    .await?;
    let factor =
        crate::domains::auth::mfa::get_factor(&state.db, auth.user_id, request.factor_id).await?;

    // Enable skip_password by default when a passkey or security key is added.
    if let Ok(mut prefs) =
        crate::domains::auth::db::fetch_user_preferences(&state.db, auth.user_id).await
    {
        prefs.skip_password = true;
        let _ = crate::domains::auth::db::update_user_preferences(&state.db, auth.user_id, &prefs)
            .await;
    }

    Ok(Json(TotpConfirmResult {
        factor,
        mfa_enabled: true,
    }))
}
