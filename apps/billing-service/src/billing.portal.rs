use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    app::BillingState,
    auth::BillingPrincipal,
    customer::get_or_create_customer,
    error::{BillingError, BillingResult},
};

#[derive(Debug, Serialize)]
pub struct PortalSessionResponse {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct BillingOverviewResponse {
    pub account_id: Uuid,
    pub customer_id: Option<String>,
    pub plan_code: String,
    pub status: String,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub cancel_at_period_end: bool,
}

pub async fn create_portal_handler(
    State(state): State<BillingState>,
    principal: BillingPrincipal,
    Path(workspace_id): Path<Uuid>,
) -> BillingResult<Json<PortalSessionResponse>> {
    principal.require_scope("billing:read")?;

    let customer_id =
        get_or_create_customer(&state.db, &state.config, workspace_id, "team", None).await?;

    let url = if state.config.stripe_secret_key.starts_with("sk_test_dummy") {
        format!(
            "{}/billing/mock-portal?customer={customer_id}",
            state.config.app_url
        )
    } else {
        create_stripe_portal(&state, &customer_id).await?
    };

    let _ = crate::audit::record_audit_event(
        &state.db,
        workspace_id,
        &principal.id().to_string(),
        "portal_session_created",
        &serde_json::json!({
            "customer_id": customer_id,
            "has_mfa": principal.has_mfa(),
        }),
    )
    .await;

    Ok(Json(PortalSessionResponse { url }))
}

pub async fn get_overview_handler(
    State(state): State<BillingState>,
    principal: BillingPrincipal,
    Path(workspace_id): Path<Uuid>,
) -> BillingResult<Json<BillingOverviewResponse>> {
    principal.require_scope("billing:read")?;

    let customer = sqlx::query_scalar!(
        "SELECT stripe_customer_id FROM billing_customers WHERE account_id = $1",
        workspace_id
    )
    .fetch_optional(&state.db)
    .await?;

    let subscription = sqlx::query!(
        r#"
        SELECT plan_code, status, current_period_end, cancel_at_period_end
        FROM billing_subscriptions
        WHERE account_id = $1
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
        workspace_id
    )
    .fetch_optional(&state.db)
    .await?;

    let (plan_code, status, current_period_end, cancel_at_period_end) = match subscription {
        Some(s) => (
            s.plan_code,
            s.status,
            s.current_period_end,
            s.cancel_at_period_end,
        ),
        None => ("free".to_string(), "active".to_string(), None, false),
    };

    Ok(Json(BillingOverviewResponse {
        account_id: workspace_id,
        customer_id: customer,
        plan_code,
        status,
        current_period_end,
        cancel_at_period_end,
    }))
}

async fn create_stripe_portal(state: &BillingState, customer_id: &str) -> BillingResult<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/billing_portal/sessions",
        state.config.stripe_api_base_url.trim_end_matches('/')
    );
    let return_url = format!("{}/billing", state.config.app_url);

    let form = [("customer", customer_id), ("return_url", &return_url)];

    let response = client
        .post(&url)
        .bearer_auth(&state.config.stripe_secret_key)
        .form(&form)
        .send()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(BillingError::Stripe(format!(
            "failed to create portal session: {err_text}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    json.get("url")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BillingError::Stripe("missing portal url in Stripe response".into()))
}

/// Used by the gRPC handler — calls the private Stripe portal helper.
pub async fn create_stripe_portal_grpc(
    state: &BillingState,
    customer_id: &str,
) -> BillingResult<String> {
    create_stripe_portal(state, customer_id).await
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.portal.tests.rs"]
mod tests;
