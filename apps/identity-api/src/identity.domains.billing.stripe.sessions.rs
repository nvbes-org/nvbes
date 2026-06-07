use uuid::Uuid;

use crate::http::error::AppError;
use nvbes_billing::stripe::StripeSession;
use nvbes_core::config::AppConfig;

pub async fn create_stripe_checkout_session(
    config: &AppConfig,
    customer_id: &str,
    owner_principal_id: Uuid,
    workspace_id: Uuid,
    plan_code: &str,
    stripe_price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<StripeSession, AppError> {
    let response = super::http::stripe_post_form(
        config,
        "/v1/checkout/sessions",
        build_checkout_session_fields(
            customer_id,
            owner_principal_id,
            workspace_id,
            plan_code,
            stripe_price_id,
            success_url,
            cancel_url,
        ),
    )
    .await?;

    stripe_session_from_response(response)
}

pub async fn create_stripe_portal_session(
    config: &AppConfig,
    customer_id: &str,
    return_url: &str,
) -> Result<StripeSession, AppError> {
    let response = super::http::stripe_post_form(
        config,
        "/v1/billing_portal/sessions",
        vec![
            ("customer".to_string(), customer_id.to_string()),
            ("return_url".to_string(), return_url.to_string()),
        ],
    )
    .await?;

    stripe_session_from_response(response)
}

pub fn build_checkout_session_fields(
    customer_id: &str,
    owner_principal_id: Uuid,
    workspace_id: Uuid,
    plan_code: &str,
    stripe_price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Vec<(String, String)> {
    vec![
        ("mode".to_string(), "subscription".to_string()),
        ("customer".to_string(), customer_id.to_string()),
        (
            "line_items[0][price]".to_string(),
            stripe_price_id.to_string(),
        ),
        ("line_items[0][quantity]".to_string(), "1".to_string()),
        ("success_url".to_string(), success_url.to_string()),
        ("cancel_url".to_string(), cancel_url.to_string()),
        ("client_reference_id".to_string(), workspace_id.to_string()),
        (
            "metadata[workspace_id]".to_string(),
            workspace_id.to_string(),
        ),
        (
            "metadata[owner_principal_id]".to_string(),
            owner_principal_id.to_string(),
        ),
        ("metadata[plan_code]".to_string(), plan_code.to_string()),
        (
            "subscription_data[metadata][workspace_id]".to_string(),
            workspace_id.to_string(),
        ),
        (
            "subscription_data[metadata][owner_principal_id]".to_string(),
            owner_principal_id.to_string(),
        ),
        (
            "subscription_data[metadata][plan_code]".to_string(),
            plan_code.to_string(),
        ),
        ("allow_promotion_codes".to_string(), "true".to_string()),
        ("automatic_tax[enabled]".to_string(), "true".to_string()),
    ]
}

fn stripe_session_from_response(response: serde_json::Value) -> Result<StripeSession, AppError> {
    let id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "stripe_response_invalid",
                "Stripe session response is missing id.",
            )
        })?;
    let url = response
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "stripe_response_invalid",
                "Stripe session response is missing url.",
            )
        })?;

    Ok(StripeSession {
        id: id.to_owned(),
        url: url.to_owned(),
    })
}
