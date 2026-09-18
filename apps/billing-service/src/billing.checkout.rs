use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::BillingState,
    auth::BillingPrincipal,
    authorization::{self, BillingAccount},
    customer::get_or_create_customer,
    error::{BillingError, BillingResult},
};

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan_code: String,
    pub account_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutSessionResponse {
    pub url: String,
    pub session_id: String,
}

pub async fn create_checkout_handler(
    State(state): State<BillingState>,
    principal: BillingPrincipal,
    Path(account): Path<BillingAccount>,
    headers: HeaderMap,
    Json(payload): Json<CreateCheckoutRequest>,
) -> BillingResult<Json<CheckoutSessionResponse>> {
    principal.require_scope("billing:checkout")?;
    let account_type = account.account_type.as_str();
    if payload
        .account_type
        .as_deref()
        .is_some_and(|value| value != account_type)
    {
        return Err(BillingError::Invalid("account_type conflicts with route"));
    }
    authorization::require(
        state.config.account_authority.as_ref(),
        principal.id(),
        &account,
    )
    .await?;
    let workspace_id = account.id;
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}_{}_{}", workspace_id, payload.plan_code, Uuid::new_v4()));
    if idempotency_key.len() > 255 || idempotency_key.is_empty() {
        return Err(BillingError::Invalid("invalid_idempotency_key"));
    }
    let idempotency_key = format!(
        "checkout_{}",
        hex::encode(Sha256::digest(format!(
            "{workspace_id}:{account_type}:{}:{idempotency_key}",
            payload.plan_code
        )))
    );

    // Check existing idempotent checkout
    if let Some((url, session_id)) = get_existing_checkout(&state.db, &idempotency_key).await? {
        return Ok(Json(CheckoutSessionResponse { url, session_id }));
    }

    // Verify plan exists
    let stripe_price_id: String = sqlx::query_scalar(
        "SELECT stripe_price_id FROM billing_plans WHERE plan_code = $1 AND is_active = true",
    )
    .bind(&payload.plan_code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(BillingError::Invalid("unknown_or_inactive_plan"))?;

    let customer_id =
        get_or_create_customer(&state.db, &state.config, workspace_id, account_type, None).await?;

    let (checkout_url, stripe_session_id) =
        if state.config.stripe_secret_key.starts_with("sk_test_dummy") {
            let sid = format!("cs_test_{}", Uuid::new_v4().simple());
            let url = format!(
                "{}/billing/mock-checkout?session_id={sid}",
                state.config.app_url
            );
            (url, sid)
        } else {
            create_stripe_checkout(
                &state,
                &customer_id,
                workspace_id,
                &payload.plan_code,
                &stripe_price_id,
                &idempotency_key,
            )
            .await?
        };

    sqlx::query(
        r#"
        INSERT INTO billing_checkout_sessions
          (idempotency_key, account_id, account_type, plan_code, stripe_session_id, stripe_customer_id, checkout_url)
          VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(&idempotency_key)
    .bind(workspace_id)
    .bind(account_type)
    .bind(&payload.plan_code)
    .bind(&stripe_session_id)
    .bind(&customer_id)
    .bind(&checkout_url)
    .execute(&state.db)
    .await?;

    let _ = crate::audit::record_audit_event(
        &state.db,
        workspace_id,
        &principal.id().to_string(),
        "checkout_session_created",
        &serde_json::json!({
            "plan_code": payload.plan_code,
            "session_id": stripe_session_id,
            "has_mfa": principal.has_mfa(),
        }),
    )
    .await;

    Ok(Json(CheckoutSessionResponse {
        url: checkout_url,
        session_id: stripe_session_id,
    }))
}

async fn get_existing_checkout(
    db: &PgPool,
    idempotency_key: &str,
) -> BillingResult<Option<(String, String)>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT checkout_url, stripe_session_id FROM billing_checkout_sessions WHERE idempotency_key = $1",
    )
    .bind(idempotency_key)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

async fn create_stripe_checkout(
    state: &BillingState,
    customer_id: &str,
    account_id: Uuid,
    plan_code: &str,
    stripe_price_id: &str,
    idempotency_key: &str,
) -> BillingResult<(String, String)> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/checkout/sessions",
        state.config.stripe_api_base_url.trim_end_matches('/')
    );

    let success_url = format!(
        "{}/billing/success?session_id={{CHECKOUT_SESSION_ID}}",
        state.config.app_url
    );
    let cancel_url = format!("{}/billing/cancel", state.config.app_url);

    let form = [
        ("customer", customer_id),
        ("mode", "subscription"),
        ("line_items[0][price]", stripe_price_id),
        ("line_items[0][quantity]", "1"),
        ("success_url", &success_url),
        ("cancel_url", &cancel_url),
        ("client_reference_id", &account_id.to_string()),
        ("metadata[account_id]", &account_id.to_string()),
        ("metadata[plan_code]", plan_code),
        (
            "subscription_data[metadata][account_id]",
            &account_id.to_string(),
        ),
        ("subscription_data[metadata][plan_code]", plan_code),
    ];

    let response = client
        .post(&url)
        .bearer_auth(&state.config.stripe_secret_key)
        .header("Idempotency-Key", idempotency_key)
        .form(&form)
        .send()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(BillingError::Stripe(format!(
            "failed to create checkout session: {err_text}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    let session_id = json
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BillingError::Stripe("missing checkout session id".into()))?;
    let checkout_url = json
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BillingError::Stripe("missing checkout url".into()))?;

    Ok((checkout_url, session_id))
}

/// Used by the gRPC handler to create a Stripe checkout session without Axum extractors.
pub async fn create_stripe_checkout_grpc(
    state: &BillingState,
    customer_id: &str,
    account_id: Uuid,
    plan_code: &str,
    idempotency_key: &str,
) -> BillingResult<(String, String)> {
    let stripe_price_id: String = sqlx::query_scalar(
        "SELECT stripe_price_id FROM billing_plans WHERE plan_code = $1 AND is_active = true",
    )
    .bind(plan_code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(BillingError::Invalid("unknown_or_inactive_plan"))?;

    create_stripe_checkout(
        state,
        customer_id,
        account_id,
        plan_code,
        &stripe_price_id,
        idempotency_key,
    )
    .await
}
