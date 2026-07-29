use crate::app::AppState;
use crate::domains::auth::mfa::recovery;
use crate::domains::auth::routes::mfa::types::RecoveryCodesGenerateRequest;
use crate::domains::auth::types::RecoveryCodesResult;
use crate::http::error::AppError;
use crate::http::middleware::jwt::{
    AuthContext,
    account_access::{self, AccountAccess, SECURITY_WRITE_SCOPE},
};
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::{delete, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/recovery-codes",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(generate_recovery_codes),
            ),
        )
        .route(
            "/factors/{factorId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                delete(remove_mfa_factor),
            ),
        )
}

#[utoipa::path(
    post,
    path = "/auth/mfa/recovery-codes",
    tag = "auth",
    request_body = RecoveryCodesGenerateRequest,
    responses(
        (status = 200, description = "Recovery codes generated", body = RecoveryCodesResult),
        (status = 400, description = "Invalid password", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn generate_recovery_codes(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<RecoveryCodesGenerateRequest>,
) -> Result<Json<RecoveryCodesResult>, AppError> {
    crate::domains::auth::verification::require_recent_phishing_resistant_step_up(
        &state.redis,
        &auth,
    )
    .await?;

    let password = request.password.as_deref().unwrap_or("");
    let result = recovery::generate_recovery(
        &state.db,
        auth.user_id,
        password,
        state.config.auth_password_pepper.as_deref(),
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/auth/mfa/factors/{factorId}",
    tag = "auth",
    responses(
        (status = 204, description = "MFA factor removed"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Factor not found", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn remove_mfa_factor(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(factor_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    crate::domains::auth::verification::require_recent_phishing_resistant_step_up(
        &state.redis,
        &auth,
    )
    .await?;
    crate::domains::auth::mfa::remove_factor(&state.db, auth.user_id, factor_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
