use crate::app::AppState;
use crate::domains::enterprise::service;
use crate::domains::enterprise::types::{
    EnterpriseAccessUpdateResponse, EnterpriseAuditReasonInput, EnterpriseBreakGlassInput,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/enterprise/users/{userId}/break-glass",
    tag = "enterprise",
    request_body = EnterpriseBreakGlassInput,
    responses(
        (status = 200, description = "Break-glass account activated", body = EnterpriseAccessUpdateResponse),
        (status = 401, description = "Step-up required", body = ErrorEnvelope),
        (status = 403, description = "Owner access required", body = ErrorEnvelope),
    ),
)]
pub async fn activate_break_glass_account(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<EnterpriseBreakGlassInput>,
) -> Result<Json<EnterpriseAccessUpdateResponse>, AppError> {
    let tenant_id = super::require_tenant(&auth)?;
    Ok(Json(
        service::activate_break_glass_account(
            &state.db,
            &state.redis,
            &auth,
            tenant_id,
            user_id,
            input,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/users/{userId}/break-glass/revoke",
    tag = "enterprise",
    request_body = EnterpriseAuditReasonInput,
    responses(
        (status = 200, description = "Break-glass account revoked", body = EnterpriseAccessUpdateResponse),
        (status = 401, description = "Step-up required", body = ErrorEnvelope),
        (status = 403, description = "Owner access required", body = ErrorEnvelope),
        (status = 404, description = "Break-glass account not found", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_break_glass_account(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<EnterpriseAuditReasonInput>,
) -> Result<Json<EnterpriseAccessUpdateResponse>, AppError> {
    let tenant_id = super::require_tenant(&auth)?;
    Ok(Json(
        service::revoke_break_glass_account(
            &state.db,
            &state.redis,
            &auth,
            tenant_id,
            user_id,
            input,
        )
        .await?,
    ))
}
