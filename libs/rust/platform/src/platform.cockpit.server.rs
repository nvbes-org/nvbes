use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;

use crate::{
    cockpit_actions::{ActionExecutionError, OperatorActionDispatcher, OperatorCommand},
    cockpit_auth::{require_operator_auth, OperatorAuthPolicy},
    cockpit_backup::BackupRestoreMonitor,
    cockpit_billing::BillingCockpitView,
    cockpit_degraded::DegradedModeRegistry,
    cockpit_email::EmailCockpitView,
    cockpit_finops::FinOpsMonitor,
    cockpit_health::HealthAggregator,
    cockpit_model::CockpitOverview,
    cockpit_trust_risk::TrustRiskCockpitView,
    cockpit_ui::render_cockpit_html,
};

#[derive(Clone)]
pub struct PlatformCockpitState {
    pub environment: String,
    pub auth_policy: Arc<OperatorAuthPolicy>,
    pub health_aggregator: Arc<HealthAggregator>,
    pub finops_monitor: Arc<FinOpsMonitor>,
}

pub fn create_platform_cockpit_router(state: PlatformCockpitState) -> Router {
    Router::new()
        .route("/health/live", get(liveness_probe))
        .route("/health/ready", get(readiness_probe))
        .route("/", get(cockpit_dashboard_ui))
        .route("/cockpit", get(cockpit_dashboard_ui))
        .route("/api/v1/overview", get(get_cockpit_overview))
        .route("/api/v1/degraded-procedures", get(get_degraded_procedures))
        .route("/api/v1/actions", post(dispatch_operator_action))
        .with_state(state)
}

async fn liveness_probe() -> impl IntoResponse {
    (StatusCode::OK, "live")
}

async fn readiness_probe() -> impl IntoResponse {
    (StatusCode::OK, "ready")
}

async fn cockpit_dashboard_ui(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    require_operator_auth(&state.auth_policy, &headers)
        .await
        .map_err(|(status, body)| (status, body).into_response())?;

    Ok(Html(render_cockpit_html(&state.environment)).into_response())
}

async fn get_cockpit_overview(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    require_operator_auth(&state.auth_policy, &headers)
        .await
        .map_err(|(status, body)| (status, body).into_response())?;

    let probes = HealthAggregator::default_probes();
    let health = state.health_aggregator.aggregate(probes.into_values().collect());

    let mut spends = HashMap::new();
    spends.insert("domain_dns", 200);
    spends.insert("compute", 250);
    spends.insert("postgres", 350);
    spends.insert("email", 80);
    spends.insert("storage_registry", 120);

    let finops = state.finops_monitor.summarize(1000, 15, 30, spends);
    let trust_risk = TrustRiskCockpitView::summarize(&[]);
    let email = EmailCockpitView::build_summary(0, 42, 42, 0, 0, 1, 0);
    let billing = BillingCockpitView::build_summary(18, 0, 0, 0, Some(Utc::now()));
    let backup_restore = BackupRestoreMonitor::evaluate(Some(Utc::now()), Some(Utc::now()), Some(45), true);

    let overview = CockpitOverview {
        environment: state.environment.clone(),
        generated_at: Utc::now(),
        health,
        outbox_jobs: crate::cockpit_model::OutboxJobsSummary {
            pending_count: 0,
            failed_count: 0,
            dead_letter_count: 0,
            unprocessed_events_count: 0,
            last_processed_at: Some(Utc::now()),
        },
        trust_risk,
        email,
        billing,
        finops,
        backup_restore,
        degraded_procedures_active: 0,
    };

    Ok(Json(overview).into_response())
}

async fn get_degraded_procedures(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    require_operator_auth(&state.auth_policy, &headers)
        .await
        .map_err(|(status, body)| (status, body).into_response())?;

    let procedures = DegradedModeRegistry::all_procedures();
    Ok(Json(procedures).into_response())
}

async fn dispatch_operator_action(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Json(cmd): Json<OperatorCommand>,
) -> Result<Response, Response> {
    require_operator_auth(&state.auth_policy, &headers)
        .await
        .map_err(|(status, body)| (status, body).into_response())?;

    match OperatorActionDispatcher::execute(cmd) {
        Ok(receipt) => Ok((StatusCode::OK, Json(receipt)).into_response()),
        Err(ActionExecutionError::InvalidReason(len)) => Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_reason",
                "message": format!("Reason must be between 3 and 300 characters (received {})", len)
            })),
        )
            .into_response()),
        Err(err) => Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "action_rejected", "message": err.to_string() })),
        )
            .into_response()),
    }
}

#[cfg(test)]
#[path = "platform.cockpit.server.tests.rs"]
mod tests;
