use serde_json::Value;

use crate::provider::ProviderPaymentMethod;

pub fn stripe_payment_method_from_object(object: &Value) -> Option<ProviderPaymentMethod> {
    let (provider_payment_method_id, card) = object_payment_method_card(object)?;

    Some(ProviderPaymentMethod {
        method_type: "card".to_string(),
        brand: optional_string(card, "brand"),
        last4: optional_string(card, "last4"),
        exp_month: optional_i16(card, "exp_month"),
        exp_year: optional_i16(card, "exp_year"),
        funding: optional_string(card, "funding"),
        issuer_country: optional_string(card, "country"),
        fingerprint: optional_string(card, "fingerprint"),
        provider_payment_method_id: Some(provider_payment_method_id),
        mandate_id: optional_string(object, "mandate"),
        mandate_status: "active".to_string(),
        reusable: true,
    })
}

fn object_payment_method_card(object: &Value) -> Option<(String, &Value)> {
    expanded_payment_method_card(object, "payment_method")
        .or_else(|| expanded_payment_method_card(object, "default_payment_method"))
        .or_else(|| payment_intent_method_details_card(object))
}

fn expanded_payment_method_card<'a>(object: &'a Value, field: &str) -> Option<(String, &'a Value)> {
    let payment_method = object.get(field)?;
    if payment_method.is_string() {
        return None;
    }
    let provider_payment_method_id = required_string(payment_method, "id")?;
    let card = payment_method.get("card")?;
    Some((provider_payment_method_id, card))
}

fn payment_intent_method_details_card(object: &Value) -> Option<(String, &Value)> {
    let provider_payment_method_id = required_string(object, "payment_method")?;
    let card = object.pointer("/payment_method_details/card")?;
    Some((provider_payment_method_id, card))
}

fn required_string(object: &Value, field: &str) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn optional_string(object: &Value, field: &str) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn optional_i16(object: &Value, field: &str) -> Option<i16> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i16::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_payment_method_from_payment_intent_extracts_non_sensitive_card_details() {
        let object = serde_json::json!({
            "id": "pi_123",
            "customer": "cus_123",
            "payment_method": "pm_123",
            "payment_method_details": {
                "card": {
                    "brand": "visa",
                    "last4": "4242",
                    "exp_month": 12,
                    "exp_year": 2030,
                    "funding": "credit",
                    "country": "FR",
                    "fingerprint": "fp_stripe_123"
                }
            }
        });

        let method = stripe_payment_method_from_object(&object).expect("card should parse");

        assert_eq!(method.method_type, "card");
        assert_eq!(method.provider_payment_method_id.as_deref(), Some("pm_123"));
        assert_eq!(method.brand.as_deref(), Some("visa"));
        assert_eq!(method.last4.as_deref(), Some("4242"));
        assert_eq!(method.exp_month, Some(12));
        assert_eq!(method.exp_year, Some(2030));
        assert_eq!(method.funding.as_deref(), Some("credit"));
        assert_eq!(method.issuer_country.as_deref(), Some("FR"));
        assert_eq!(method.fingerprint.as_deref(), Some("fp_stripe_123"));
        assert!(method.reusable);
    }

    #[test]
    fn stripe_payment_method_from_expanded_payment_method_extracts_card_details() {
        let object = serde_json::json!({
            "id": "sub_123",
            "customer": "cus_123",
            "default_payment_method": {
                "id": "pm_456",
                "card": {
                    "brand": "mastercard",
                    "last4": "4444",
                    "exp_month": 1,
                    "exp_year": 2031
                }
            }
        });

        let method = stripe_payment_method_from_object(&object).expect("card should parse");

        assert_eq!(method.provider_payment_method_id.as_deref(), Some("pm_456"));
        assert_eq!(method.brand.as_deref(), Some("mastercard"));
        assert_eq!(method.last4.as_deref(), Some("4444"));
        assert_eq!(method.exp_month, Some(1));
        assert_eq!(method.exp_year, Some(2031));
    }

    #[test]
    fn stripe_payment_method_ignores_unexpanded_payment_method_without_card_details() {
        let object = serde_json::json!({
            "id": "sub_123",
            "customer": "cus_123",
            "default_payment_method": "pm_456"
        });

        assert!(stripe_payment_method_from_object(&object).is_none());
    }
}
