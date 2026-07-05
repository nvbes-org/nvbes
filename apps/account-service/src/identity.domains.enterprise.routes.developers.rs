use axum::{
    Json,
    extract::{Extension, Path, State},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::enterprise::service;
use crate::domains::enterprise::types::EnterpriseDevelopersResponse;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use super::require_tenant;

pub(super) async fn list_developers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseDevelopersResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_developers(&state.db, &auth, tenant_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/developers/credentials/{credentialId}/revoke",
    tag = "enterprise",
    params(("credentialId" = Uuid, Path, description = "Developer secret version ID")),
    responses(
        (status = 200, description = "Developer secret revoked", body = EnterpriseDevelopersResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developers grant or admin elevation required", body = ErrorEnvelope),
        (status = 404, description = "Developer secret not found", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_developer_secret(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(credential_id): Path<Uuid>,
) -> Result<Json<EnterpriseDevelopersResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::revoke_developer_secret(&state.db, &state.redis, &auth, tenant_id, credential_id)
            .await?,
    ))
}
