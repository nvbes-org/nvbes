use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::fraud::{CheckoutFraudDecision, NetworkThreatLevel};
use crate::provider::{ProviderCode, ProviderPayment};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingPspFraudSignal {
    pub provider: &'static str,
    pub provider_payment_id: Option<String>,
    pub provider_payment_status: &'static str,
    pub plan_code: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub score: u8,
    pub decision: CheckoutFraudDecision,
    pub network_threat: NetworkThreatLevel,
    pub labels: Vec<String>,
    pub reasons: Vec<String>,
    pub metadata: Value,
}

pub fn stripe_invoice_payment_failed_signal(object: &Value) -> BillingPspFraudSignal {
    let provider_payment_id = string_at(object, "/payment_intent")
        .or_else(|| string_field(object, "payment_intent"))
        .or_else(|| string_field(object, "id"));
    let amount_minor = object
        .get("amount_due")
        .and_then(Value::as_i64)
        .or_else(|| object.get("amount_remaining").and_then(Value::as_i64));
    let currency = string_field(object, "currency").map(|value| value.to_ascii_uppercase());
    let plan_code = string_at(object, "/metadata/plan_code")
        .or_else(|| string_at(object, "/subscription_details/metadata/plan_code"));
    let stripe_radar_risk_level = string_at(object, "/charge/outcome/risk_level")
        .or_else(|| string_at(object, "/payment_intent/latest_charge/outcome/risk_level"));
    let stripe_radar_risk_score = int_at(object, "/charge/outcome/risk_score")
        .or_else(|| int_at(object, "/payment_intent/latest_charge/outcome/risk_score"));
    let three_d_secure_result = string_at(
        object,
        "/payment_intent/payment_method_details/card/three_d_secure/result",
    )
    .or_else(|| {
        string_at(
            object,
            "/charge/payment_method_details/card/three_d_secure/result",
        )
    });
    let provider_payment_method_id = string_at(object, "/payment_intent/payment_method")
        .or_else(|| string_at(object, "/charge/payment_method"))
        .or_else(|| string_field(object, "payment_method"));

    let mut labels = vec![
        "payment:provider_failed".to_string(),
        "provider:stripe".to_string(),
    ];
    let mut reasons = vec!["provider_payment_failed".to_string()];
    if let Some(level) = stripe_radar_risk_level.as_deref() {
        labels.push(format!("stripe_radar:risk_level_{level}"));
        if matches!(level, "elevated" | "highest") {
            reasons.push("stripe_radar_risk".to_string());
        }
    }
    if let Some(result) = three_d_secure_result.as_deref() {
        labels.push(format!("three_d_secure:{result}"));
        if matches!(result, "failed" | "not_supported" | "processing_error") {
            reasons.push("three_d_secure_unsuccessful".to_string());
        }
    }

    BillingPspFraudSignal {
        provider: ProviderCode::Stripe.as_str(),
        provider_payment_id,
        provider_payment_status: "failed",
        plan_code,
        amount_minor,
        currency,
        score: psp_score("failed", stripe_radar_risk_level.as_deref()),
        decision: CheckoutFraudDecision::Monitor,
        network_threat: NetworkThreatLevel::Elevated,
        labels,
        reasons,
        metadata: json!({
            "provider": ProviderCode::Stripe.as_str(),
            "provider_payment_status": "failed",
            "stripe_failure_code": string_at(object, "/last_payment_error/code")
                .or_else(|| string_at(object, "/last_finalization_error/code")),
            "stripe_radar_risk_level": stripe_radar_risk_level,
            "stripe_radar_risk_score": stripe_radar_risk_score,
            "stripe_network_status": string_at(object, "/charge/outcome/network_status")
                .or_else(|| string_at(object, "/payment_intent/latest_charge/outcome/network_status")),
            "three_d_secure_result": three_d_secure_result,
            "provider_payment_method_id": provider_payment_method_id
        }),
    }
}

pub fn mollie_payment_signal(payment: &ProviderPayment) -> Option<BillingPspFraudSignal> {
    if !matches!(
        payment.status.as_str(),
        "failed" | "disputed" | "charged_back" | "canceled"
    ) {
        return None;
    }
    let labels = vec![
        format!("payment:provider_{}", payment.status),
        "provider:mollie".to_string(),
    ];
    let reasons = vec![format!("provider_payment_{}", payment.status)];

    Some(BillingPspFraudSignal {
        provider: ProviderCode::Mollie.as_str(),
        provider_payment_id: Some(payment.provider_payment_id.clone()),
        provider_payment_status: provider_payment_status(&payment.status),
        plan_code: payment.plan_code.clone(),
        amount_minor: Some(payment.amount_minor),
        currency: Some(payment.currency.to_ascii_uppercase()),
        score: psp_score(provider_payment_status(&payment.status), None),
        decision: CheckoutFraudDecision::Monitor,
        network_threat: NetworkThreatLevel::Elevated,
        labels,
        reasons,
        metadata: json!({
            "provider": ProviderCode::Mollie.as_str(),
            "provider_payment_status": provider_payment_status(&payment.status),
            "provider_payment_id": payment.provider_payment_id,
            "provider_payment_method": payment.payment_method.as_ref().map(|method| method.method_type.as_str()),
            "provider_payment_method_id": payment.payment_method.as_ref().and_then(|method| method.provider_payment_method_id.as_deref()),
            "card_issuer_country": payment.payment_method.as_ref().and_then(|method| method.issuer_country.as_deref()),
            "card_funding": payment.payment_method.as_ref().and_then(|method| method.funding.as_deref()),
            "mandate_status": payment.payment_method.as_ref().map(|method| method.mandate_status.as_str())
        }),
    })
}

fn provider_payment_status(status: &str) -> &'static str {
    match status {
        "disputed" | "charged_back" => "disputed",
        "canceled" => "canceled",
        _ => "failed",
    }
}

fn psp_score(status: &str, stripe_radar_risk_level: Option<&str>) -> u8 {
    match (status, stripe_radar_risk_level) {
        ("disputed", _) => 75,
        ("failed", Some("highest")) => 70,
        ("failed", Some("elevated")) => 60,
        ("failed", _) => 45,
        ("canceled", _) => 30,
        _ => 20,
    }
}

fn string_field(object: &Value, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn string_at(object: &Value, pointer: &str) -> Option<String> {
    object
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn int_at(object: &Value, pointer: &str) -> Option<i64> {
    object.pointer(pointer).and_then(Value::as_i64)
}

#[cfg(test)]
#[path = "psp_signals.tests.rs"]
mod tests;
