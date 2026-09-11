use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::{
    app::BillingState,
    auth::BillingPrincipal,
    authorization::{self, BillingAccount},
    customer::get_or_create_customer,
    error::{BillingError, BillingResult},
};

#[derive(Debug, Serialize)]
pub struct PortalSessionResponse {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct BillingOverviewResponse {
    pub account_id: uuid::Uuid,
    pub customer_id: Option<String>,
    pub plan_code: String,
    pub status: String,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub cancel_at_period_end: bool,
}

pub async fn create_portal_handler(
    State(state): State<BillingState>,
    principal: BillingPrincipal,
    Path(account): Path<BillingAccount>,
) -> BillingResult<Json<PortalSessionResponse>> {
    principal.require_scope("billing:read")?;
    authorization::require(
        state.config.account_authority.as_ref(),
        principal.id(),
        &account,
    )
    .await?;
    let workspace_id = account.id;

    let customer_id = get_or_create_customer(
        &state.db,
        &state.config,
        workspace_id,
        account.account_type.as_str(),
        None,
    )
    .await?;

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
    Path(account): Path<BillingAccount>,
) -> BillingResult<Json<BillingOverviewResponse>> {
    principal.require_scope("billing:read")?;
    authorization::require(
        state.config.account_authority.as_ref(),
        principal.id(),
        &account,
    )
    .await?;
    let workspace_id = account.id;

    let customer: Option<String> = sqlx::query_scalar(
        "SELECT stripe_customer_id FROM billing_customers WHERE account_id = $1 AND account_type = $2",
    )
    .bind(workspace_id)
    .bind(account.account_type.as_str())
    .fetch_optional(&state.db)
    .await?;

    let subscription: Option<(String, String, Option<chrono::DateTime<chrono::Utc>>, bool)> =
        sqlx::query_as(
            r#"
        SELECT plan_code, status, current_period_end, cancel_at_period_end
        FROM billing_subscriptions
        WHERE account_id = $1 AND account_type = $2
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
        )
        .bind(workspace_id)
        .bind(account.account_type.as_str())
        .fetch_optional(&state.db)
        .await?;

    let (plan_code, status, current_period_end, cancel_at_period_end) = match subscription {
        Some((plan, st, end, cancel)) => (plan, st, end, cancel),
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
