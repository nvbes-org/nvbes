use super::http::{mollie_get_json, mollie_post_json};
use crate::http::error::AppError;
use nvbes_billing::provider::{
    ProviderCheckout, ProviderCheckoutInput, ProviderCode, ProviderPayment,
};
use nvbes_core::config::AppConfig;

pub async fn create_mollie_payment(
    config: &AppConfig,
    input: &ProviderCheckoutInput,
) -> Result<ProviderCheckout, AppError> {
    let payload = build_mollie_payment_payload(input)?;
    let response = mollie_post_json(config, "/v2/payments", payload).await?;
    checkout_from_mollie_payment(response)
}

pub async fn fetch_mollie_payment(
    config: &AppConfig,
    provider_payment_id: &str,
) -> Result<ProviderPayment, AppError> {
    let response = mollie_get_json(config, &format!("/v2/payments/{provider_payment_id}")).await?;
    payment_from_mollie_response(response)
}

pub fn build_mollie_payment_payload(
    input: &ProviderCheckoutInput,
) -> Result<serde_json::Value, AppError> {
    if input.amount_minor <= 0 {
        return Err(AppError::bad_request(
            "invalid_mollie_amount",
            "Mollie payment amount must be greater than zero.",
        ));
    }
    let mut payload = serde_json::json!({
        "amount": {
            "currency": input.currency,
            "value": minor_units_to_mollie_amount(input.amount_minor),
        },
        "description": format!("nvbes billing {}", input.tenant_id),
        "redirectUrl": input.success_url,
        "metadata": {
            "tenant_id": input.tenant_id,
            "provider_customer_id": input.provider_customer_id,
        }
    });
    if let Some(webhook_url) = &input.webhook_url {
        payload["webhookUrl"] = serde_json::Value::String(webhook_url.clone());
    }
    Ok(payload)
}

pub fn checkout_from_mollie_payment(
    response: serde_json::Value,
) -> Result<ProviderCheckout, AppError> {
    let checkout_id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal("mollie_response_invalid", "Mollie payment is missing id.")
        })?;
    let url = response
        .pointer("/_links/checkout/href")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "mollie_response_invalid",
                "Mollie payment is missing checkout link.",
            )
        })?;

    Ok(ProviderCheckout {
        provider: ProviderCode::Mollie,
        checkout_id: checkout_id.to_string(),
        url: url.to_string(),
    })
}

pub fn payment_from_mollie_response(
    response: serde_json::Value,
) -> Result<ProviderPayment, AppError> {
    let provider_payment_id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal("mollie_response_invalid", "Mollie payment is missing id.")
        })?;
    let status = response
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let amount = response.get("amount").ok_or_else(|| {
        AppError::internal(
            "mollie_response_invalid",
            "Mollie payment is missing amount.",
        )
    })?;
    let currency = amount
        .get("currency")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "mollie_response_invalid",
                "Mollie payment amount is missing currency.",
            )
        })?;
    let value = amount
        .get("value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "mollie_response_invalid",
                "Mollie payment amount is missing value.",
            )
        })?;

    Ok(ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: provider_payment_id.to_string(),
        status: mollie_status_to_provider_status(status).to_string(),
        amount_minor: mollie_amount_to_minor_units(value)?,
        currency: currency.to_string(),
    })
}

fn minor_units_to_mollie_amount(amount_minor: i64) -> String {
    format!("{}.{:02}", amount_minor / 100, amount_minor % 100)
}

fn mollie_amount_to_minor_units(value: &str) -> Result<i64, AppError> {
    let (major, minor) = value.split_once('.').ok_or_else(|| {
        AppError::internal(
            "mollie_response_invalid",
            "Mollie amount must include decimals.",
        )
    })?;
    let major = major.parse::<i64>().map_err(|_| {
        AppError::internal(
            "mollie_response_invalid",
            "Mollie amount major part is invalid.",
        )
    })?;
    let minor = minor.parse::<i64>().map_err(|_| {
        AppError::internal(
            "mollie_response_invalid",
            "Mollie amount minor part is invalid.",
        )
    })?;
    Ok(major * 100 + minor)
}

fn mollie_status_to_provider_status(status: &str) -> &'static str {
    match status {
        "paid" => "captured",
        "authorized" => "authorized",
        "failed" | "expired" => "failed",
        "canceled" => "canceled",
        "refunded" => "refunded",
        "charged_back" => "disputed",
        "open" | "pending" => "pending",
        _ => "pending",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mollie_payment_payload_uses_amount_redirect_and_webhook() {
        let payload = build_mollie_payment_payload(&ProviderCheckoutInput {
            tenant_id: "tenant_1".to_string(),
            provider_customer_id: "cst_1".to_string(),
            amount_minor: 1234,
            currency: "EUR".to_string(),
            success_url: "https://app.example/success".to_string(),
            cancel_url: "https://app.example/cancel".to_string(),
            webhook_url: Some("https://api.example/webhook".to_string()),
        })
        .expect("payload should build");

        assert_eq!(payload["amount"]["value"], "12.34");
        assert_eq!(payload["amount"]["currency"], "EUR");
        assert_eq!(payload["redirectUrl"], "https://app.example/success");
        assert_eq!(payload["webhookUrl"], "https://api.example/webhook");
    }

    #[test]
    fn mollie_checkout_link_is_extracted_from_payment_response() {
        let checkout = checkout_from_mollie_payment(serde_json::json!({
            "id": "tr_123",
            "_links": { "checkout": { "href": "https://www.mollie.com/checkout/select-method/abc" } }
        }))
        .expect("checkout should parse");

        assert_eq!(checkout.provider, ProviderCode::Mollie);
        assert_eq!(checkout.checkout_id, "tr_123");
    }

    #[test]
    fn mollie_payment_status_maps_to_provider_payment() {
        let payment = payment_from_mollie_response(serde_json::json!({
            "id": "tr_123",
            "status": "paid",
            "amount": { "currency": "EUR", "value": "12.34" }
        }))
        .expect("payment should parse");

        assert_eq!(payment.status, "captured");
        assert_eq!(payment.amount_minor, 1234);
    }
}
