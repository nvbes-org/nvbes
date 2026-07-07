use crate::{
    app::AppState,
    domains::developer::{
        grpc,
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperSandboxResponse, DeveloperSandboxTenantSummary, UpsertDeveloperSandboxInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};
use axum::{Extension, Json, extract::State};

pub async fn get_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperSandboxResponse>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(grpc::get_sandbox(tenant_id, auth.user_id).await?))
}

pub async fn upsert_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<UpsertDeveloperSandboxInput>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(
        grpc::upsert_sandbox(tenant_id, auth.user_id, input).await?,
    ))
}

pub async fn reset_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(grpc::reset_sandbox(tenant_id, auth.user_id).await?))
}
