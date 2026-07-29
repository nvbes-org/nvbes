use crate::app::AppState;
use crate::http::middleware::jwt::{
    AuthContext,
    account_access::{self, AccountAccess, SECURITY_WRITE_SCOPE},
};
use axum::{
    Router,
    extract::{Extension, State},
    routing::delete,
};
use uuid::Uuid;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/factors/{factorId}",
        account_access::protected_method(
            state,
            AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
            delete(remove_mfa_factor),
        ),
    )
}

async fn remove_mfa_factor(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(factor_id): axum::extract::Path<Uuid>,
) -> Result<axum::http::StatusCode, crate::http::error::AppError> {
    crate::domains::auth::verification::require_recent_phishing_resistant_step_up(
        &state.redis,
        &auth,
    )
    .await?;
    crate::domains::auth::mfa::remove_factor(&state.db, auth.user_id(), factor_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
