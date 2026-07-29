use crate::app::AppState;
use crate::domains::auth::routes::mfa::types::{
    WebauthnRegisterFinishRequest, WebauthnRegisterStartRequest,
};
use crate::domains::auth::types::{
    TotpConfirmResult, WebauthnAuthStartResult, WebauthnRegisterStartResult,
};
use crate::domains::auth::webauthn as webauthn_mod;
use crate::http::error::AppError;
use crate::http::middleware::jwt::{
    AuthContext,
    account_access::{self, AccountAccess, SECURITY_WRITE_SCOPE},
};
use axum::{
    Json, Router,
    extract::{Extension, State},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/webauthn/start",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(webauthn_auth_start),
            ),
        )
        .route(
            "/webauthn/register/start",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(begin_webauthn_enrollment),
            ),
        )
        .route(
            "/webauthn/register/finish",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(confirm_webauthn_enrollment),
            ),
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
    crate::domains::auth::verification::require_passkey_enrollment_step_up(
        &state.db,
        &state.redis,
        &auth,
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

    // Only discoverable passkeys can be offered before an account is identified.
    if factor.kind.as_deref() == Some("passkey")
        && let Ok(mut prefs) =
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
