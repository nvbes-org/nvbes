use serde_json::Value;
use uuid::Uuid;

use super::db::BillingStateRecord;
use crate::http::error::AppError;
use nvbes_billing::stripe::{StripeCustomer, StripeSession, StripeWebhookEvent};
use nvbes_core::config::AppConfig;

pub async fn create_stripe_customer(
    config: &AppConfig,
    record: &BillingStateRecord,
) -> Result<StripeCustomer, AppError> {
    let response = stripe_post_form(
        config,
        "/v1/customers",
        vec![
            ("email".to_string(), record.owner_email.clone()),
            ("name".to_string(), record.workspace_name.clone()),
            (
                "metadata[workspace_id]".to_string(),
                record.workspace_id.to_string(),
            ),
            (
                "metadata[owner_principal_id]".to_string(),
                record.owner_principal_id.to_string(),
            ),
        ],
    )
    .await?;

    let id = response.get("id").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe customer response is missing id.",
        )
    })?;

    Ok(StripeCustomer { id: id.to_string() })
}

pub async fn create_stripe_checkout_session(
    config: &AppConfig,
    customer_id: &str,
    owner_principal_id: Uuid,
    workspace_id: Uuid,
    plan_code: &str,
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<StripeSession, AppError> {
    let fields = vec![
        ("customer".to_string(), customer_id.to_string()),
        ("mode".to_string(), "subscription".to_string()),
        ("success_url".to_string(), success_url.to_string()),
        ("cancel_url".to_string(), cancel_url.to_string()),
        ("line_items[0][price]".to_string(), price_id.to_string()),
        ("line_items[0][quantity]".to_string(), "1".to_string()),
        ("automatic_tax[enabled]".to_string(), "true".to_string()),
        (
            "billing_address_collection".to_string(),
            "required".to_string(),
        ),
        ("customer_update[name]".to_string(), "auto".to_string()),
        ("customer_update[address]".to_string(), "auto".to_string()),
        (
            "metadata[workspace_id]".to_string(),
            workspace_id.to_string(),
        ),
        (
            "metadata[owner_principal_id]".to_string(),
            owner_principal_id.to_string(),
        ),
        (
            "subscription_data[metadata][workspace_id]".to_string(),
            workspace_id.to_string(),
        ),
        (
            "subscription_data[metadata][owner_principal_id]".to_string(),
            owner_principal_id.to_string(),
        ),
        ("metadata[plan_code]".to_string(), plan_code.to_string()),
    ];

    let response = stripe_post_form(config, "/v1/checkout/sessions", fields).await?;

    let id = response.get("id").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe checkout session response is missing id.",
        )
    })?;

    let url = response.get("url").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe checkout session response is missing url.",
        )
    })?;

    Ok(StripeSession {
        id: id.to_string(),
        url: url.to_string(),
    })
}

pub async fn create_stripe_portal_session(
    config: &AppConfig,
    customer_id: &str,
    return_url: &str,
) -> Result<StripeSession, AppError> {
    let response = stripe_post_form(
        config,
        "/v1/billing_portal/sessions",
        vec![
            ("customer".to_string(), customer_id.to_string()),
            ("return_url".to_string(), return_url.to_string()),
        ],
    )
    .await?;

    let id = response.get("id").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe portal session response is missing id.",
        )
    })?;

    let url = response.get("url").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe portal session response is missing url.",
        )
    })?;

    Ok(StripeSession {
        id: id.to_string(),
        url: url.to_string(),
    })
}

async fn stripe_post_form(
    config: &AppConfig,
    endpoint: &str,
    fields: Vec<(String, String)>,
) -> Result<Value, AppError> {
    let secret_key = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::internal("stripe_not_configured", "Stripe is not configured."))?;
    let url = format!("{}{}", config.stripe_api_base_url, endpoint);

    let client = nvbes_core::security::pinned_http_client();
    let request = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", secret_key))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&fields);
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|e| {
            AppError::internal(
                "stripe_request_failed",
                &format!("Stripe request failed: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(
            "stripe_api_error",
            &format!("Stripe API error ({}): {}", status, body),
        ));
    }

    let json: Value = response.json().await.map_err(|e| {
        AppError::internal(
            "stripe_response_invalid",
            &format!("Failed to parse Stripe response: {}", e),
        )
    })?;

    if let Some(error) = json.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Unknown Stripe error");
        return Err(AppError::internal("stripe_error", message));
    }

    Ok(json)
}

pub fn verify_stripe_signature(
    config: &AppConfig,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<(), AppError> {
    let webhook_secret = config.stripe_webhook_secret.as_deref().ok_or_else(|| {
        AppError::internal("stripe_not_configured", "Stripe webhook is not configured.")
    })?;

    nvbes_billing::verify_stripe_signature(webhook_secret, signature_header, payload).map_err(
        |code| {
            let message = match code {
                "missing_stripe_signature" => "Stripe signature header is missing.",
                "stale_stripe_signature" => "Stripe signature has expired.",
                _ => "Stripe signature is invalid.",
            };
            AppError::bad_request(code, message)
        },
    )
}

pub fn parse_stripe_event(payload: &[u8]) -> Result<StripeWebhookEvent, AppError> {
    nvbes_billing::parse_stripe_event(payload).ok_or_else(|| {
        AppError::bad_request(
            "stripe_event_invalid",
            "Stripe event is missing type or id.",
        )
    })
}
