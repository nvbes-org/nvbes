use crate::app::AppState;
use crate::domains::auth::mfa;
use crate::domains::auth::routes::mfa::types::EmailMfaSetupRequest;
use crate::domains::auth::types::{EmailAddressesResult, TotpConfirmResult};
use crate::http::error::AppError;
use crate::http::middleware::jwt::{AuthContext, jwt_auth_middleware};
use axum::{
    Json, Router,
    extract::{Extension, State},
    routing::{get, post},
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/email/eligible",
            get(list_email_mfa_eligible).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
        .route(
            "/email/setup",
            post(setup_email_mfa).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
}

pub(crate) async fn list_email_mfa_eligible(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EmailAddressesResult>, AppError> {
    Ok(Json(
        mfa::email::eligible_emails(&state.db, auth.user_id).await?,
    ))
}

pub(crate) async fn setup_email_mfa(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<EmailMfaSetupRequest>,
) -> Result<Json<TotpConfirmResult>, AppError> {
    crate::domains::auth::verification::require_recent_step_up(
        &state.redis,
        &auth,
        Some(nvbes_core::auth::Aal::Aal2),
    )
    .await?;
    Ok(Json(
        mfa::email::setup_email_factor(&state.db, auth.user_id, request.email_id).await?,
    ))
}
