use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        sandbox, service,
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
    Ok(Json(DeveloperSandboxResponse {
        sandbox: sandbox::find_sandbox(&state.db, tenant_id).await?,
    }))
}

pub async fn upsert_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<UpsertDeveloperSandboxInput>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(
        sandbox::upsert_sandbox(&state.db, tenant_id, input.data_profile.as_deref()).await?,
    ))
}

pub async fn reset_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(sandbox::reset_sandbox(&state.db, tenant_id).await?))
}
