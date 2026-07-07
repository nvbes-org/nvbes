use axum::{Extension, Json, extract::State};

use crate::{
    app::AppState,
    domains::developer::{
        grpc,
        rbac::DeveloperPermission,
        service,
        types::{DeveloperHealthChecksResponse, RunDeveloperHealthChecksResponse},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_health_checks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperHealthChecksResponse>, AppError> {
    let tenant_id = service::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleHealthChecksRead,
    )
    .await?;
    Ok(Json(
        grpc::list_health_checks(tenant_id, auth.user_id).await?,
    ))
}

pub async fn run_health_checks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<RunDeveloperHealthChecksResponse>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::HealthChecksRun).await?;
    Ok(Json(
        grpc::run_health_checks(tenant_id, auth.user_id).await?,
    ))
}
