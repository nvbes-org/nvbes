use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{authorize_billing_admin_access, require_tenant_id};
use crate::app::AppState;
use crate::domains::billing::service_admin::{
    BillingAdminMutationResult, BillingAdminSearchResult, BillingGraceOverrideInput,
    BillingManualCompInput, create_manual_compensation, override_grace_period,
    search_billing_admin,
};
use crate::http::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/billing/admin/search",
            get(search_billing_admin_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/grace-overrides",
            post(override_grace_period_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/manual-compensations",
            post(create_manual_compensation_route),
        )
}

#[derive(Debug, Deserialize)]
pub struct BillingAdminSearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize)]
pub struct GraceOverrideRequest {
    pub subscription_id: Option<Uuid>,
    pub grace_days: i64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ManualCompRequest {
    pub amount_minor: i64,
    pub currency: String,
    pub direction: String,
    pub reason: String,
}

pub async fn search_billing_admin_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<BillingAdminSearchQuery>,
) -> Result<Json<Vec<BillingAdminSearchResult>>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;
    let results = search_billing_admin(&state.db, tenant_id, &query.q).await?;
    Ok(Json(results))
}

pub async fn override_grace_period_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<GraceOverrideRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = override_grace_period(
        &state.db,
        BillingGraceOverrideInput {
            tenant_id,
            workspace_id,
            actor_principal_id: access.auth.user_id,
            subscription_id: request.subscription_id,
            grace_days: request.grace_days,
            reason: request.reason,
        },
    )
    .await?;
    Ok(Json(result))
}

pub async fn create_manual_compensation_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ManualCompRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = create_manual_compensation(
        &state.db,
        BillingManualCompInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            amount_minor: request.amount_minor,
            currency: request.currency,
            direction: request.direction,
            reason: request.reason,
        },
    )
    .await?;
    Ok(Json(result))
}
