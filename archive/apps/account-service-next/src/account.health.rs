use axum::{Json, extract::State};
use serde::Serialize;

use crate::{app::AppState, error::AppError};

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Account readiness database check failed");
            AppError::service_unavailable(
                "database_unavailable",
                "The Account database is unavailable.",
            )
        })?;
    Ok(Json(HealthResponse { status: "ready" }))
}
