use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    app::BillingAppState,
    auth::{BillingWorkspacePermission, authorize_billing_workspace},
    domains::public_workspace_portal_lists::{
        get_billing_cards, get_billing_invoices, get_billing_subscriptions,
    },
    http::error::AppError,
};

pub fn router() -> Router<BillingAppState> {
    Router::new()
        .route("/billing/portal/capabilities", get(get_portal_capabilities))
        .route(
            "/workspaces/{workspaceId}/billing/overview",
            get(get_billing_overview),
        )
        .route(
            "/workspaces/{workspaceId}/billing/usage",
            get(get_billing_usage),
        )
        .route(
            "/workspaces/{workspaceId}/billing/entitlements",
            get(get_entitlements),
        )
        .route(
            "/workspaces/{workspaceId}/billing/checkout",
            post(create_checkout_session),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal",
            post(create_portal_session),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal/view",
            get(get_portal_view),
        )
        .route(
            "/workspaces/{workspaceId}/billing/invoices",
            get(get_billing_invoices),
        )
        .route(
            "/workspaces/{workspaceId}/billing/cards",
            get(get_billing_cards),
        )
        .route(
            "/workspaces/{workspaceId}/billing/subscriptions",
            get(get_billing_subscriptions),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal/invoices/{invoiceId}/pdf",
            get(download_invoice_pdf),
        )
}

pub async fn get_portal_capabilities()
-> Json<nvbes_billing::portal_views::BillingPortalCapabilities> {
    Json(nvbes_billing::portal_views::billing_portal_capabilities())
}

pub async fn get_billing_overview(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::BillingOverviewResponse>, AppError> {
    authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    Ok(Json(
        nvbes_billing::fetch_workspace_billing_overview(&state.db, workspace_id).await?,
    ))
}

pub async fn get_billing_usage(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::BillingUsageResponse>, AppError> {
    authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    Ok(Json(
        nvbes_billing::fetch_workspace_billing_usage(&state.db, workspace_id).await?,
    ))
}

pub async fn get_entitlements(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::ProductEntitlementsView>, AppError> {
    authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    Ok(Json(
        nvbes_billing::fetch_workspace_entitlements(&state.db, workspace_id).await?,
    ))
}

pub async fn create_checkout_session(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<nvbes_billing::types::CreateCheckoutInput>,
) -> Result<Json<nvbes_billing::types::CheckoutSessionResponse>, AppError> {
    let auth = authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Manage,
    )
    .await?;
    nvbes_billing::action_rate_limits::enforce_billing_action_rate_limits(
        &state.rate_limiter,
        workspace_id,
        auth.principal_id,
        "checkout",
    )
    .await?;
    let plan_code = input.plan_code.clone();
    let response = nvbes_billing::checkout_sessions::create_billing_checkout_session(
        &state.db,
        &state.config,
        nvbes_billing::checkout_sessions::CreateBillingCheckoutSessionInput {
            workspace_id,
            actor_principal_id: auth.principal_id,
            checkout: input,
            ip: nvbes_core::http::client_ip::client_ip(&headers),
            trusted_country_header: country_header(&headers).map(ToOwned::to_owned),
            user_agent: user_agent(&headers),
        },
    )
    .await?;

    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::workspace(
            "billing.checkout_started",
            workspace_id,
        )
        .correlation(
            headers
                .get("X-PostHog-Distinct-Id")
                .and_then(|value| value.to_str().ok()),
            headers
                .get("X-PostHog-Session-Id")
                .and_then(|value| value.to_str().ok()),
        )
        .property("plan_code", plan_code),
    );

    Ok(Json(response))
}

pub async fn create_portal_session(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<nvbes_billing::types::CreatePortalInput>,
) -> Result<Json<nvbes_billing::types::PortalSessionResponse>, AppError> {
    let auth = authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Manage,
    )
    .await?;
    nvbes_billing::action_rate_limits::enforce_billing_action_rate_limits(
        &state.rate_limiter,
        workspace_id,
        auth.principal_id,
        "portal",
    )
    .await?;
    Ok(Json(
        nvbes_billing::portal_actions::create_billing_portal_session(
            &state.db,
            &state.config,
            nvbes_billing::portal_actions::CreateBillingPortalSessionInput {
                workspace_id,
                actor_principal_id: auth.principal_id,
                portal: input,
                ip: nvbes_core::http::client_ip::client_ip(&headers),
                user_agent: user_agent(&headers),
            },
        )
        .await?,
    ))
}

pub async fn get_portal_view(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::portal_views::BillingPortalView>, AppError> {
    authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    Ok(Json(
        nvbes_billing::portal_views::fetch_portal_view(&state.db, workspace_id).await?,
    ))
}

pub async fn download_invoice_pdf(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path((workspace_id, invoice_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    authorize_billing_workspace(
        &state.db,
        &headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    let download =
        nvbes_billing::invoice_pdf::fetch_invoice_pdf(&state.db, workspace_id, invoice_id).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", download.filename),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        download.body,
    ))
}

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn country_header(headers: &HeaderMap) -> Option<&str> {
    ["CF-IPCountry", "X-Vercel-IP-Country", "X-AppEngine-Country"]
        .iter()
        .find_map(|name| headers.get(*name).and_then(|value| value.to_str().ok()))
}

#[cfg(test)]
mod tests {
    use super::router;

    #[test]
    fn public_workspace_billing_routes_are_registered() {
        let router = router();
        let _ = router;
    }
}
