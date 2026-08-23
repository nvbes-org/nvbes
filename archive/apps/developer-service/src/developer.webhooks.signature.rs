use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::http::error::AppError;

type HmacSha256 = Hmac<Sha256>;

pub const WEBHOOK_SIGNATURE_VERSION: &str = "v1";
pub const WEBHOOK_SIGNATURE_TOLERANCE_SECONDS: i64 = 300;

/// Decision applied before replaying a Developer webhook delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookReplayDecision {
    Replayable,
    NotReplayable,
}

pub fn webhook_signature_header(
    signing_secret: &str,
    timestamp: i64,
    event_id: Uuid,
    payload: &[u8],
) -> String {
    let signature = sign_webhook_payload(signing_secret, timestamp, event_id, payload);
    format!("t={timestamp},{WEBHOOK_SIGNATURE_VERSION}={signature}")
}

pub fn sign_webhook_payload(
    signing_secret: &str,
    timestamp: i64,
    event_id: Uuid,
    payload: &[u8],
) -> String {
    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(event_id.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

pub fn signature_timestamp_is_fresh(now: i64, timestamp: i64) -> bool {
    let age = now - timestamp;
    (0..=WEBHOOK_SIGNATURE_TOLERANCE_SECONDS).contains(&age)
}

pub fn classify_webhook_replay(status: &str) -> WebhookReplayDecision {
    match status {
        "failed" | "pending" => WebhookReplayDecision::Replayable,
        "delivered" | "replayed" => WebhookReplayDecision::NotReplayable,
        _ => WebhookReplayDecision::NotReplayable,
    }
}

pub fn ensure_replayable_delivery_status(status: &str) -> Result<(), AppError> {
    if classify_webhook_replay(status) == WebhookReplayDecision::Replayable {
        return Ok(());
    }

    Err(AppError::conflict(
        "webhook_delivery_not_replayable",
        "Only failed or pending webhook deliveries can be replayed",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_webhook_signature_binds_event_timestamp_and_payload() {
        let event_id = Uuid::parse_str("018f7b1e-2b1c-73bd-a657-65df58aa1001").unwrap();
        let signature = webhook_signature_header(
            "whsec_test",
            1_732_000_000,
            event_id,
            br#"{"type":"user.created"}"#,
        );

        assert!(signature.starts_with("t=1732000000,v1="));
        assert_ne!(
            signature,
            webhook_signature_header(
                "whsec_test",
                1_732_000_001,
                event_id,
                br#"{"type":"user.created"}"#,
            )
        );
        assert_ne!(
            signature,
            webhook_signature_header(
                "whsec_test",
                1_732_000_000,
                Uuid::new_v4(),
                br#"{"type":"user.created"}"#,
            )
        );
        assert_ne!(
            signature,
            webhook_signature_header(
                "whsec_test",
                1_732_000_000,
                event_id,
                br#"{"type":"login.failed"}"#,
            )
        );
    }

    #[test]
    fn developer_webhook_signature_timestamp_rejects_replay_window() {
        assert!(signature_timestamp_is_fresh(1_732_000_100, 1_732_000_000));
        assert!(!signature_timestamp_is_fresh(1_732_000_301, 1_732_000_000));
        assert!(!signature_timestamp_is_fresh(1_732_000_000, 1_732_000_001));
    }

    #[test]
    fn developer_webhook_replay_decision_is_idempotency_guard() {
        assert_eq!(
            classify_webhook_replay("failed"),
            WebhookReplayDecision::Replayable
        );
        assert_eq!(
            classify_webhook_replay("pending"),
            WebhookReplayDecision::Replayable
        );
        assert_eq!(
            classify_webhook_replay("delivered"),
            WebhookReplayDecision::NotReplayable
        );
        assert_eq!(
            classify_webhook_replay("replayed"),
            WebhookReplayDecision::NotReplayable
        );
    }
}
