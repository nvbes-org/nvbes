use crate::{
    cockpit_auth::OperatorAuthPolicy, operations_context::ContextClient, operations_routes,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlatformCockpitState {
    pub environment: String,
    pub auth_policy: Arc<OperatorAuthPolicy>,
    pub db: PgPool,
    pub context: Arc<ContextClient>,
}

pub fn create_platform_cockpit_router(state: PlatformCockpitState) -> Router {
    Router::new()
        .route("/health/live", get(|| async { "live" }))
        .route("/health/ready", get(readiness))
        .route("/", get(overview))
        .route("/cockpit", get(overview))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/actions", post(unsupported_action))
        .route("/api/v1/commands", post(operations_routes::command))
        .route("/api/v1/cases", get(operations_routes::cases))
        .route("/api/v1/cases/{id}", get(operations_routes::case_context))
        .route("/api/v1/audits", get(operations_routes::audits))
        .route("/api/v1/costs", get(operations_routes::costs))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state)
}

async fn readiness(State(state): State<PlatformCockpitState>) -> StatusCode {
    match sqlx::query("SELECT 1 FROM operations_audit LIMIT 1")
        .execute(&state.db)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn overview(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let actor = state.auth_policy.authenticate_headers(&headers)?;
    Ok(Json(
        json!({"environment":state.environment,"operator":actor.operator_id,"role":actor.role,
        "services":state.context.snapshot().await, "cases":"/api/v1/cases", "audits":"/api/v1/audits",
        "commands":"/api/v1/commands", "costs":"/api/v1/costs?month=YYYY-MM-01",
        "domain_mutations":"unavailable", "backup_restore":"not_verified",
        "panels":"removed", "note":"Use case observations and owner APIs; simulated cockpit panels are not served"}),
    ))
}

async fn unsupported_action(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    state.auth_policy.authenticate_headers(&headers)?;
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({"error":"domain_action_unavailable","executed":false})),
    ))
}

#[cfg(test)]
#[path = "platform.cockpit.server.tests.rs"]
mod tests;
