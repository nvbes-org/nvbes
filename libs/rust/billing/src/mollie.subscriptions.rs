use serde_json::Value;

use crate::mollie::{MollieProviderError, minor_units_to_mollie_amount};
use crate::provider::{ProviderCode, ProviderSubscription, ProviderSubscriptionInput};
use nvbes_core::config::AppConfig;

pub async fn create_mollie_subscription(
    config: &AppConfig,
    input: &ProviderSubscriptionInput,
) -> Result<ProviderSubscription, MollieProviderError> {
    let response = crate::mollie::mollie_post_json(
        config,
        &format!("/v2/customers/{}/subscriptions", input.provider_customer_id),
        build_mollie_subscription_payload(input)?,
    )
    .await?;
    subscription_from_mollie_response(&input.provider_customer_id, response)
}

pub fn build_mollie_subscription_payload(
    input: &ProviderSubscriptionInput,
) -> Result<Value, MollieProviderError> {
    if input.amount_minor <= 0 {
        return Err(MollieProviderError::InvalidRequest {
            code: "invalid_mollie_amount",
            message: "Mollie subscription amount must be greater than zero.",
        });
    }
    if input.interval.trim().is_empty() {
        return Err(MollieProviderError::InvalidRequest {
            code: "invalid_mollie_interval",
            message: "Mollie subscription interval is required.",
        });
    }

    let mut payload = serde_json::json!({
        "amount": {
            "currency": input.currency,
            "value": minor_units_to_mollie_amount(input.amount_minor),
        },
        "interval": input.interval,
        "description": input.description,
        "metadata": {
            "provider_customer_id": input.provider_customer_id,
        }
    });
    if let Some(start_date) = &input.start_date {
        payload["startDate"] = Value::String(start_date.clone());
    }
    if let Some(webhook_url) = &input.webhook_url {
        payload["webhookUrl"] = Value::String(webhook_url.clone());
    }
    Ok(payload)
}

pub fn subscription_from_mollie_response(
    provider_customer_id: &str,
    response: Value,
) -> Result<ProviderSubscription, MollieProviderError> {
    let provider_subscription_id =
        response
            .get("id")
            .and_then(Value::as_str)
            .ok_or(MollieProviderError::InvalidResponse {
                code: "mollie_response_invalid",
                message: "Mollie subscription is missing id.",
            })?;
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("pending");

    Ok(ProviderSubscription {
        provider: ProviderCode::Mollie,
        provider_subscription_id: provider_subscription_id.to_string(),
        status: status.to_string(),
        provider_customer_id: provider_customer_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mollie_subscription_payload_uses_amount_interval_start_date_and_webhook() {
        let payload = build_mollie_subscription_payload(&ProviderSubscriptionInput {
            provider_customer_id: "cst_123".to_string(),
            amount_minor: 2500,
            currency: "EUR".to_string(),
            interval: "1 month".to_string(),
            description: "nvbes monthly subscription".to_string(),
            start_date: Some("2026-08-01".to_string()),
            webhook_url: Some("https://api.example/webhooks/mollie".to_string()),
        })
        .expect("payload should build");

        assert_eq!(payload["amount"]["value"], "25.00");
        assert_eq!(payload["interval"], "1 month");
        assert_eq!(payload["startDate"], "2026-08-01");
        assert_eq!(payload["webhookUrl"], "https://api.example/webhooks/mollie");
    }

    #[test]
    fn mollie_subscription_response_extracts_subscription_id() {
        let subscription = subscription_from_mollie_response(
            "cst_123",
            serde_json::json!({
                "id": "sub_123",
                "status": "active"
            }),
        )
        .expect("subscription should parse");

        assert_eq!(subscription.provider, ProviderCode::Mollie);
        assert_eq!(subscription.provider_subscription_id, "sub_123");
        assert_eq!(subscription.provider_customer_id, "cst_123");
        assert_eq!(subscription.status, "active");
    }
}
