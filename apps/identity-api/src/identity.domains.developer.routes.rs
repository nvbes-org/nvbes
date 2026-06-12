use axum::{Extension, Json, Router, extract::State, routing::get};
use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    http::{
        error::AppError,
        middleware::jwt::{AuthContext, jwt_auth_middleware},
    },
};

use super::{rbac_db, types::DeveloperMeResponse};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/developer/me",
        get(me).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        )),
    )
}

#[utoipa::path(
    get,
    path = "/developer/me",
    tag = "developer",
    responses(
        (status = 200, description = "Current developer portal access", body = DeveloperMeResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant context required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperMeResponse>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Tenant context is required for developer portal access.",
        )
    })?;
    let roles = rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id).await?;
    let permissions = rbac_db::permissions_for_roles(&roles);

    Ok(Json(DeveloperMeResponse::new(
        tenant_id,
        roles,
        permissions,
    )))
}
