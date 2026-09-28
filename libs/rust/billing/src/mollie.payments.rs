use serde_json::Value;

use crate::mollie::{MollieProviderError, minor_units_to_mollie_amount};
use crate::provider::{
    ProviderCheckout, ProviderCheckoutInput, ProviderCode, ProviderPayment, ProviderPaymentMethod,
};

pub fn build_mollie_payment_payload(
    input: &ProviderCheckoutInput,
) -> Result<Value, MollieProviderError> {
    if input.amount_minor <= 0 {
        return Err(MollieProviderError::InvalidRequest {
            code: "invalid_mollie_amount",
            message: "Mollie payment amount must be greater than zero.",
        });
    }
    let mut payload = serde_json::json!({
        "amount": {
            "currency": input.currency,
            "value": minor_units_to_mollie_amount(input.amount_minor),
        },
        "description": format!("nvbes billing {}", input.tenant_id),
        "redirectUrl": input.success_url,
        "customerId": input.provider_customer_id,
        "sequenceType": "first",
        "metadata": {
            "tenant_id": input.tenant_id,
            "provider_customer_id": input.provider_customer_id,
            "plan_code": input.plan_code,
        }
    });
    if let Some(metadata) = payload.get_mut("metadata").and_then(Value::as_object_mut) {
        for (key, value) in &input.fraud_metadata {
            metadata.insert(key.clone(), Value::String(value.clone()));
        }
    }
    if let Some(webhook_url) = &input.webhook_url {
        payload["webhookUrl"] = Value::String(webhook_url.clone());
    }
    Ok(payload)
}

pub fn checkout_from_mollie_payment(
    response: Value,
) -> Result<ProviderCheckout, MollieProviderError> {
    let checkout_id =
        response
            .get("id")
            .and_then(Value::as_str)
            .ok_or(MollieProviderError::InvalidResponse {
                code: "mollie_response_invalid",
                message: "Mollie payment is missing id.",
            })?;
    let url = response
        .pointer("/_links/checkout/href")
        .and_then(Value::as_str)
        .ok_or(MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie payment is missing checkout link.",
        })?;

    Ok(ProviderCheckout {
        provider: ProviderCode::Mollie,
        checkout_id: checkout_id.to_string(),
        url: url.to_string(),
    })
}

pub fn payment_from_mollie_response(
    response: Value,
) -> Result<ProviderPayment, MollieProviderError> {
    let provider_payment_id =
        response
            .get("id")
            .and_then(Value::as_str)
            .ok_or(MollieProviderError::InvalidResponse {
                code: "mollie_response_invalid",
                message: "Mollie payment is missing id.",
            })?;
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let amount = response
        .get("amount")
        .ok_or(MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie payment is missing amount.",
        })?;
    let currency = amount.get("currency").and_then(Value::as_str).ok_or(
        MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie payment amount is missing currency.",
        },
    )?;
    let value = amount.get("value").and_then(Value::as_str).ok_or(
        MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie payment amount is missing value.",
        },
    )?;

    Ok(ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: provider_payment_id.to_string(),
        provider_customer_id: response
            .get("customerId")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_subscription_id: response
            .get("subscriptionId")
            .and_then(Value::as_str)
            .map(str::to_string),
        plan_code: response
            .pointer("/metadata/plan_code")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: mollie_status_to_provider_status(status).to_string(),
        amount_minor: mollie_amount_to_minor_units(value)?,
        currency: currency.to_string(),
        payment_method: payment_method_from_mollie_response(&response, status),
    })
}

fn payment_method_from_mollie_response(
    response: &Value,
    mollie_status: &str,
) -> Option<ProviderPaymentMethod> {
    let details = response.get("details")?;
    let card_label = details
        .get("cardLabel")
        .and_then(Value::as_str)
        .map(str::to_string);
    let last4 = details
        .get("cardNumber")
        .and_then(Value::as_str)
        .and_then(card_last4);
    let fingerprint = details
        .get("cardFingerprint")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mandate_id = response
        .get("mandateId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let provider_payment_method_id = mandate_id.clone().or_else(|| fingerprint.clone());

    if provider_payment_method_id.is_none()
        && card_label.is_none()
        && last4.is_none()
        && fingerprint.is_none()
    {
        return None;
    }

    let (exp_month, exp_year) = details
        .get("cardExpiryDate")
        .and_then(Value::as_str)
        .and_then(parse_card_expiry)
        .unwrap_or((None, None));
    let reusable = mandate_id.is_some();

    Some(ProviderPaymentMethod {
        method_type: "card".to_string(),
        brand: card_label.map(|value| value.to_ascii_lowercase()),
        last4,
        exp_month,
        exp_year,
        funding: details
            .get("cardFunding")
            .and_then(Value::as_str)
            .map(str::to_string),
        issuer_country: details
            .get("cardCountryCode")
            .and_then(Value::as_str)
            .map(str::to_string),
        fingerprint,
        provider_payment_method_id,
        mandate_id,
        mandate_status: if reusable && mollie_status == "paid" {
            "valid".to_string()
        } else {
            "unknown".to_string()
        },
        reusable,
    })
}

fn card_last4(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.len() >= 4 {
        Some(digits[digits.len() - 4..].to_string())
    } else {
        None
    }
}

fn parse_card_expiry(value: &str) -> Option<(Option<i16>, Option<i16>)> {
    let (month, year) = value.split_once('/')?;
    let month = month.parse::<i16>().ok()?;
    let mut year = year.parse::<i16>().ok()?;
    if year < 100 {
        year += 2000;
    }
    Some((Some(month), Some(year)))
}

fn mollie_amount_to_minor_units(value: &str) -> Result<i64, MollieProviderError> {
    let (major, minor) = value
        .split_once('.')
        .ok_or(MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie amount must include decimals.",
        })?;
    let major = major
        .parse::<i64>()
        .map_err(|_| MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie amount major part is invalid.",
        })?;
    let minor = minor
        .parse::<i64>()
        .map_err(|_| MollieProviderError::InvalidResponse {
            code: "mollie_response_invalid",
            message: "Mollie amount minor part is invalid.",
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
#[path = "mollie.payments.tests.rs"]
mod tests;
