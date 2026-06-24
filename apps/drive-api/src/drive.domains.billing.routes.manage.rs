use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use std::time::Instant;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::ProductAnalyticsEvent;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::{
        error::AppError,
        request::{client_ip, country_header, user_agent},
    },
};

use crate::domains::billing::service::{
    BillingOverviewResponse, BillingUsageResponse, CheckoutSessionResponse, CreateCheckoutInput,
    CreatePortalInput, InvoiceEstimateResponse, PortalSessionResponse,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/billing", get(get_billing))
        .route(
            "/workspaces/{workspaceId}/billing/checkout",
            post(create_checkout_session),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal",
            post(create_portal_session),
        )
        .route("/workspaces/{workspaceId}/billing/usage", get(get_usage))
        .route(
            "/workspaces/{workspaceId}/billing/invoice-estimate",
            get(get_invoice_estimate),
        )
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/billing",
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
async fn get_billing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingOverviewResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::billing::get_billing(&state.db, &access).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/billing/checkout",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateCheckoutInput,
    responses(
        (status = 200, description = "Checkout session created", body = CheckoutSessionResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_checkout_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateCheckoutInput>,
) -> Result<Json<CheckoutSessionResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ManageBilling,
        ResourceContext::default(),
    )
    .await?;

    let plan_code = request.plan_code.clone();
    let result = crate::domains::billing::create_checkout_session(
        &state.db,
        &state.config,
        &access,
        request,
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
    request_body = CreatePortalInput,
    responses(
        (status = 200, description = "Portal session created", body = PortalSessionResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_portal_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreatePortalInput>,
) -> Result<Json<PortalSessionResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ManageBilling,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::billing::create_portal_session(
        &state.config,
        &state.db,
        &access,
        request,
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
async fn get_usage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingUsageResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::billing::get_usage(&state.db, &access).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/billing/invoice-estimate",
    tag = "billing",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Invoice estimate", body = InvoiceEstimateResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn get_invoice_estimate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<InvoiceEstimateResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::billing::get_invoice_estimate(&state.db, &access).await?;
    Ok(Json(result))
}
