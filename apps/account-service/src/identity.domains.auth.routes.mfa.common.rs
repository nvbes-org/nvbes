use crate::app::AppState;
use crate::http::middleware::jwt::{jwt_auth_middleware, AuthContext};
use axum::{extract::{Extension, State}, routing::delete, Router};
use uuid::Uuid;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/factors/{factorId}",
        delete(remove_mfa_factor).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        )),
    )
}

async fn remove_mfa_factor(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(factor_id): axum::extract::Path<Uuid>,
) -> Result<axum::http::StatusCode, crate::http::error::AppError> {
    crate::domains::auth::verification::require_recent_step_up(
        &state.redis,
        &auth,
        Some(nvbes_core::auth::Aal::Aal2),
    )
    .await?;
    crate::domains::auth::mfa::remove_factor(
        &state.db,
        auth.user_id(),
        factor_id,
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
