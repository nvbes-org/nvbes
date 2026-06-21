use crate::http::error::AppError;
use nvbes_billing::provider::{ProviderCode, ProviderWebhookEvent};
use sha2::{Digest, Sha256};

pub fn mollie_payment_webhook_id(payload: &serde_json::Value) -> Option<&str> {
    payload.get("id").and_then(serde_json::Value::as_str)
}

pub fn verify_mollie_classic_webhook(payload: &[u8]) -> Result<ProviderWebhookEvent, AppError> {
    let payload_hash = nvbes_billing::hex_encode(&Sha256::digest(payload));
    let body = std::str::from_utf8(payload).map_err(|_| {
        AppError::bad_request("invalid_mollie_webhook", "Webhook payload is not UTF-8.")
    })?;
    let provider_payment_id = parse_mollie_webhook_id(body).ok_or_else(|| {
        AppError::bad_request(
            "invalid_mollie_webhook",
            "Mollie webhook payload is missing payment id.",
        )
    })?;

    Ok(ProviderWebhookEvent {
        provider: ProviderCode::Mollie,
        provider_event_id: provider_payment_id.clone(),
        event_type: "payment.updated".to_string(),
        payload_hash,
        payload_summary: serde_json::json!({ "payment_id": provider_payment_id }),
        signature_valid: true,
        raw_retention_class: "summary_only".to_string(),
    })
}

fn parse_mollie_webhook_id(body: &str) -> Option<String> {
    if body.trim_start().starts_with('{') {
        let json = serde_json::from_str::<serde_json::Value>(body).ok()?;
        return json
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
    }
    body.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "id").then(|| value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mollie_webhook_payment_id_is_extracted_from_payload() {
        let payload = serde_json::json!({ "id": "tr_123" });
        assert_eq!(mollie_payment_webhook_id(&payload), Some("tr_123"));
    }

    #[test]
    fn mollie_classic_webhook_is_id_only_and_fetch_required() {
        let event = verify_mollie_classic_webhook(b"id=tr_123").expect("webhook should parse");
        assert_eq!(event.provider, ProviderCode::Mollie);
        assert_eq!(event.provider_event_id, "tr_123");
        assert_eq!(event.event_type, "payment.updated");
        assert!(event.signature_valid);
    }
}
