use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::mollie::MollieProviderError;
use crate::provider::{ProviderCode, ProviderWebhookEvent};
use crate::shared::hex_encode;

pub fn verify_mollie_classic_webhook(
    payload: &[u8],
) -> Result<ProviderWebhookEvent, MollieProviderError> {
    let payload_hash = hex_encode(&Sha256::digest(payload));
    let body = std::str::from_utf8(payload).map_err(|_| MollieProviderError::InvalidWebhook {
        code: "invalid_mollie_webhook",
        message: "Webhook payload is not UTF-8.",
    })?;
    let provider_payment_id =
        parse_mollie_webhook_id(body).ok_or(MollieProviderError::InvalidWebhook {
            code: "invalid_mollie_webhook",
            message: "Mollie webhook payload is missing payment id.",
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

pub fn mollie_payment_webhook_id(payload: &Value) -> Option<&str> {
    payload.get("id").and_then(Value::as_str)
}

fn parse_mollie_webhook_id(body: &str) -> Option<String> {
    if body.trim_start().starts_with('{') {
        let json = serde_json::from_str::<Value>(body).ok()?;
        return json.get("id").and_then(Value::as_str).map(str::to_string);
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
