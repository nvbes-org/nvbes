use reqwest::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use super::types::BillingStateRecord;
use crate::http::error::AppError;
use nvbes_billing::shared::form_encode;
use nvbes_billing::stripe::{StripeCustomer, StripeSession, StripeWebhookEvent};
use nvbes_core::config::AppConfig;

pub async fn create_stripe_customer(
    config: &AppConfig,
    record: &BillingStateRecord,
) -> Result<StripeCustomer, AppError> {
    let response = stripe_post_form(config, "/v1/customers", build_customer_fields(record)).await?;

    let id = response.get("id").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe customer response is missing id.",
        )
    })?;

    Ok(StripeCustomer { id: id.to_owned() })
}

pub fn build_customer_fields(record: &BillingStateRecord) -> Vec<(String, String)> {
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
    ]
}

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
    let response = stripe_post_form(
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
    let response = stripe_post_form(
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

fn stripe_session_from_response(response: Value) -> Result<StripeSession, AppError> {
    let id = response.get("id").and_then(Value::as_str).ok_or_else(|| {
        AppError::internal(
            "stripe_response_invalid",
            "Stripe session response is missing id.",
        )
    })?;
    let url = response.get("url").and_then(Value::as_str).ok_or_else(|| {
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

pub async fn stripe_post_form(
    config: &AppConfig,
    path: &str,
    fields: Vec<(String, String)>,
) -> Result<Value, AppError> {
    let secret_key = config.stripe_secret_key.as_deref().ok_or_else(|| {
        AppError::conflict(
            "stripe_not_configured",
            "NVBES_STRIPE_SECRET_KEY must be configured before billing actions.",
        )
    })?;
    let url = format!(
        "{}{}",
        config.stripe_api_base_url.trim_end_matches('/'),
        path
    );
    let body = form_encode(fields);
    let client = nvbes_core::security::pinned_http_client();

    let request = client
        .post(url)
        .bearer_auth(secret_key)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body);
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|error| {
            AppError::internal(
                "stripe_request_failed",
                format!("Stripe request failed: {error}").as_str(),
            )
        })?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        AppError::internal(
            "stripe_response_failed",
            format!("Stripe response failed: {error}").as_str(),
        )
    })?;

    if !status.is_success() {
        return Err(stripe_error(status, &body));
    }

    serde_json::from_str(&body).map_err(|error| {
        AppError::internal(
            "stripe_response_invalid",
            format!("Stripe response is not valid JSON: {error}").as_str(),
        )
    })
}

fn stripe_error(status: StatusCode, body: &str) -> AppError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Stripe returned HTTP {status}."));

    if status == StatusCode::BAD_REQUEST {
        AppError::bad_request("stripe_request_rejected", &message)
    } else {
        AppError::conflict("stripe_request_rejected", &message)
    }
}

pub fn parse_stripe_event(payload: &[u8]) -> Result<StripeWebhookEvent, AppError> {
    nvbes_billing::parse_stripe_event(payload).ok_or_else(|| {
        AppError::bad_request(
            "invalid_webhook_payload",
            "Webhook payload is not valid JSON or missing required fields.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{build_checkout_session_fields, build_customer_fields};
    use crate::domains::billing::types::BillingStateRecord;
    use uuid::Uuid;

    #[test]
    fn build_checkout_session_fields_includes_workspace_and_subscription_metadata() {
        let workspace_id = Uuid::new_v4();
        let owner_principal_id = Uuid::new_v4();
        let fields = build_checkout_session_fields(
            "cus_123",
            owner_principal_id,
            workspace_id,
            "pro",
            "price_123",
            "https://app.example.com/billing/success",
            "https://app.example.com/billing/cancel",
        );

        assert!(fields.iter().any(|(key, value)| {
            key == "client_reference_id" && value == &workspace_id.to_string()
        }));
        assert!(fields.iter().any(|(key, value)| {
            key == "metadata[workspace_id]" && value == &workspace_id.to_string()
        }));
        assert!(fields.iter().any(|(key, value)| {
            key == "metadata[owner_principal_id]" && value == &owner_principal_id.to_string()
        }));
        assert!(fields.iter().any(|(key, value)| {
            key == "subscription_data[metadata][workspace_id]" && value == &workspace_id.to_string()
        }));
        assert!(fields.iter().any(|(key, value)| {
            key == "subscription_data[metadata][owner_principal_id]"
                && value == &owner_principal_id.to_string()
        }));
    }

    #[test]
    fn build_customer_fields_includes_workspace_metadata() {
        let record = BillingStateRecord {
            workspace_id: Uuid::new_v4(),
            workspace_name: "Acme".to_string(),
            owner_principal_id: Uuid::new_v4(),
            owner_email: "owner@example.com".to_string(),
            trial_ends_at: None,
            plan_code: "trial".to_string(),
            included_storage_gb: 0,
            included_users: 0,
            retention_days: 30,
            max_share_links: 100,
            audit_level: "standard".to_string(),
            max_share_link_ttl_days: 90,
            subscription_status: "trialing".to_string(),
            billing_customer_id: None,
            billing_subscription_id: None,
            current_period_start: None,
            current_period_end: None,
            stripe_customer_id: None,
            billing_email: None,
            country: None,
            customer_type: "b2b".to_string(),
            vat_number: None,
            tax_exempt_status: None,
            used_storage_bytes: 0,
            bandwidth_out_bytes_month: 0,
            active_user_count: 0,
        };

        let fields = build_customer_fields(&record);

        assert!(fields.iter().any(|(key, value)| {
            key == "metadata[workspace_id]" && value == &record.workspace_id.to_string()
        }));
        assert!(fields.iter().any(|(key, value)| {
            key == "metadata[owner_principal_id]" && value == &record.owner_principal_id.to_string()
        }));
    }
}

// form_encode and percent_encode moved to nvbes-billing
