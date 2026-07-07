use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        account_billing::{
            grpc::{
                billing_client, checkout_session_from_grpc, grpc_error, overview_from_grpc,
                portal_from_grpc, portal_session_from_grpc, request_context,
            },
            types::{
                AccountBillingOverview, AccountBillingPortalView, AccountBillingSession,
                CreateAccountBillingPortalRequest, CreateAccountCheckoutRequest,
            },
        },
        authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    },
    grpc_pb::nvbes::billing::v1::{
        CreateCheckoutRequest, CreatePortalRequest, GetBillingOverviewRequest,
        GetBillingPortalRequest,
    },
    http::error::AppError,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/account/billing/workspaces/{workspaceId}/overview",
            get(get_account_billing_overview),
        )
        .route(
            "/account/billing/workspaces/{workspaceId}/portal",
            get(get_account_billing_portal).post(create_account_billing_portal),
        )
        .route(
            "/account/billing/workspaces/{workspaceId}/checkout",
            post(create_account_checkout),
        )
}

#[utoipa::path(
    get,
    path = "/account/billing/workspaces/{workspaceId}/overview",
    tag = "account-billing",
    params(("workspaceId" = Uuid, Path, description = "Workspace ID")),
    responses(
        (status = 200, description = "Account billing overview", body = AccountBillingOverview),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 502, description = "Billing service unavailable", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_account_billing_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<AccountBillingOverview>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;
    let context = request_context(&headers, &access);
    let mut client = billing_client(&state.billing_grpc_endpoint).await?;
    let response = client
        .get_billing_overview(GetBillingOverviewRequest {
            context: Some(context),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(Json(overview_from_grpc(response)))
}

#[utoipa::path(
    get,
    path = "/account/billing/workspaces/{workspaceId}/portal",
    tag = "account-billing",
    params(("workspaceId" = Uuid, Path, description = "Workspace ID")),
    responses(
        (status = 200, description = "Account billing portal view", body = AccountBillingPortalView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 502, description = "Billing service unavailable", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_account_billing_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<AccountBillingPortalView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;
    let context = request_context(&headers, &access);
    let mut client = billing_client(&state.billing_grpc_endpoint).await?;
    let response = client
        .get_billing_portal(GetBillingPortalRequest {
            context: Some(context),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(Json(portal_from_grpc(response)))
}

#[utoipa::path(
    post,
    path = "/account/billing/workspaces/{workspaceId}/checkout",
    tag = "account-billing",
    params(("workspaceId" = Uuid, Path, description = "Workspace ID")),
    request_body = CreateAccountCheckoutRequest,
    responses(
        (status = 200, description = "Account billing checkout session", body = AccountBillingSession),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 502, description = "Billing service unavailable", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_account_checkout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateAccountCheckoutRequest>,
) -> Result<Json<AccountBillingSession>, AppError> {
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
    let context = request_context(&headers, &access);
    let mut client = billing_client(&state.billing_grpc_endpoint).await?;
    let response = client
        .create_checkout(CreateCheckoutRequest {
            context: Some(context),
            workspace_id: workspace_id.to_string(),
            plan_code: request.plan_code,
            success_url: request.success_url.unwrap_or_default(),
            cancel_url: request.cancel_url.unwrap_or_default(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(Json(checkout_session_from_grpc(response)))
}

#[utoipa::path(
    post,
    path = "/account/billing/workspaces/{workspaceId}/portal",
    tag = "account-billing",
    params(("workspaceId" = Uuid, Path, description = "Workspace ID")),
    request_body = CreateAccountBillingPortalRequest,
    responses(
        (status = 200, description = "Account billing portal session", body = AccountBillingSession),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 502, description = "Billing service unavailable", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_account_billing_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateAccountBillingPortalRequest>,
) -> Result<Json<AccountBillingSession>, AppError> {
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
    let context = request_context(&headers, &access);
    let mut client = billing_client(&state.billing_grpc_endpoint).await?;
    let response = client
        .create_portal(CreatePortalRequest {
            context: Some(context),
            workspace_id: workspace_id.to_string(),
            return_url: request.return_url.unwrap_or_default(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(Json(portal_session_from_grpc(response)))
}
