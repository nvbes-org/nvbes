use uuid::Uuid;

use super::{StripeProviderError, StripeSession};
use nvbes_core::config::AppConfig;

#[derive(Debug, Clone)]
pub struct StripeCheckoutSessionParams<'a> {
    pub customer_id: &'a str,
    pub owner_principal_id: Uuid,
    pub workspace_id: Uuid,
    pub plan_code: &'a str,
    pub stripe_price_id: &'a str,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub fraud_metadata: &'a [(String, String)],
}

pub async fn create_stripe_checkout_session(
    config: &AppConfig,
    params: StripeCheckoutSessionParams<'_>,
) -> Result<StripeSession, StripeProviderError> {
    let response = super::stripe_post_form(
        config,
        "/v1/checkout/sessions",
        build_checkout_session_fields(params),
    )
    .await?;

    stripe_session_from_response(response)
}

pub async fn create_stripe_portal_session(
    config: &AppConfig,
    customer_id: &str,
    return_url: &str,
) -> Result<StripeSession, StripeProviderError> {
    let response = super::stripe_post_form(
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
    params: StripeCheckoutSessionParams<'_>,
) -> Vec<(String, String)> {
    let StripeCheckoutSessionParams {
        customer_id,
        owner_principal_id,
        workspace_id,
        plan_code,
        stripe_price_id,
        success_url,
        cancel_url,
        fraud_metadata,
    } = params;
    let mut fields = vec![
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
    ];
    for (key, value) in fraud_metadata {
        fields.push((format!("metadata[{key}]"), value.clone()));
        fields.push((format!("subscription_data[metadata][{key}]"), value.clone()));
    }
    if fraud_metadata_requests_step_up(fraud_metadata) {
        fields.push((
            "payment_method_options[card][request_three_d_secure]".to_string(),
            "challenge".to_string(),
        ));
    }
    fields
}

fn fraud_metadata_requests_step_up(fraud_metadata: &[(String, String)]) -> bool {
    fraud_metadata
        .iter()
        .any(|(key, value)| key == "enforcement_action" && value == "step_up")
}

fn stripe_session_from_response(
    response: serde_json::Value,
) -> Result<StripeSession, StripeProviderError> {
    let id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            StripeProviderError::ResponseInvalid(
                "Stripe session response is missing id.".to_string(),
            )
        })?;
    let url = response
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            StripeProviderError::ResponseInvalid(
                "Stripe session response is missing url.".to_string(),
            )
        })?;

    Ok(StripeSession {
        id: id.to_owned(),
        url: url.to_owned(),
    })
}
