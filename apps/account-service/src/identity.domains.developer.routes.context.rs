use axum::{Extension, Json, extract::State};

use crate::{
    app::AppState,
    domains::developer::{service, types},
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn get_context(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<types::DeveloperContextResponse>, AppError> {
    Ok(Json(
        service::get_developer_context(&state.db, &auth).await?,
    ))
}

pub async fn get_overview(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<types::DeveloperOverviewResponse>, AppError> {
    Ok(Json(
        service::get_developer_overview(&state.db, &auth).await?,
    ))
}
