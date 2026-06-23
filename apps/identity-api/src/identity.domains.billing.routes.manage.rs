use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action};
use crate::domains::billing::service::{
    self, BillingOverviewResponse, BillingUsageResponse, CheckoutSessionResponse,
    CreateCheckoutInput, CreatePortalInput, PortalSessionResponse,
};
use crate::http::error::AppError;
use crate::http::request::{client_ip, country_header, user_agent};
use nvbes_product_analytics::ProductAnalyticsEvent;
use std::time::Instant;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/billing/overview",
            get(get_billing_overview),
        )
        .route(
            "/workspaces/{workspaceId}/billing/checkout",
            post(create_checkout),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal",
            post(create_portal),
        )
        .route(
            "/workspaces/{workspaceId}/billing/usage",
            get(get_billing_usage),
        )
        .route(
            "/workspaces/{workspaceId}/billing/entitlements",
            get(get_entitlements),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateCheckoutRequest {
    plan_code: String,
    success_url: Option<String>,
    cancel_url: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreatePortalRequest {
    return_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/billing/overview",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Billing overview", body = BillingOverviewResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_billing_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingOverviewResponse>, AppError> {
    authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(service::get_billing(&state.db, workspace_id).await?))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/billing/checkout",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateCheckoutRequest,
    responses(
        (status = 200, description = "Checkout session created", body = CheckoutSessionResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_checkout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateCheckoutRequest>,
) -> Result<Json<CheckoutSessionResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ManageBilling,
        ResourceContext::default(),
    )
    .await?;

    let plan_code = request.plan_code;
    let result = service::create_checkout_session(
        &state.rate_limiter,
        &state.db,
        &state.redis,
        &state.config,
        &access,
        CreateCheckoutInput {
            plan_code: plan_code.clone(),
            success_url: request.success_url,
            cancel_url: request.cancel_url,
        },
        client_ip(&headers),
        country_header(
            &headers,
            &["CF-IPCountry", "X-Vercel-IP-Country", "X-AppEngine-Country"],
        )
        .map(ToOwned::to_owned),
        user_agent(&headers),
    )
    .await;

    match &result {
        Ok(_) => state.observability.record_billing_operation(
            "checkout_session",
            "success",
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_billing_operation(
            "checkout_session",
            "failure",
            started_at.elapsed(),
        ),
    }

    if result.is_ok() {
        state.product_analytics.capture(
            ProductAnalyticsEvent::workspace_for_user(
                "billing.checkout_started",
                access.auth.user_id,
                workspace_id,
            )
            .property("plan_code", plan_code),
        );
    }

    Ok(Json(result?))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/billing/portal",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreatePortalRequest,
    responses(
        (status = 200, description = "Portal session created", body = PortalSessionResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreatePortalRequest>,
) -> Result<Json<PortalSessionResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ManageBilling,
        ResourceContext::default(),
    )
    .await?;

    let result = service::create_portal_session(
        &state.rate_limiter,
        &state.db,
        &state.redis,
        &state.config,
        &access,
        CreatePortalInput {
            return_url: request.return_url,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await;

    match &result {
        Ok(_) => state.observability.record_billing_operation(
            "portal_session",
            "success",
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_billing_operation(
            "portal_session",
            "failure",
            started_at.elapsed(),
        ),
    }

    Ok(Json(result?))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/billing/usage",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Billing usage", body = BillingUsageResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_billing_usage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingUsageResponse>, AppError> {
    authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(service::get_usage(&state.db, workspace_id).await?))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/billing/entitlements",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Product entitlements", body = nvbes_billing::types::ProductEntitlementsView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_entitlements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::ProductEntitlementsView>, AppError> {
    authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    let billing = service::get_billing(&state.db, workspace_id).await?;
    Ok(Json(billing.entitlements))
}
