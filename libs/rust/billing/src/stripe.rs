use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::shared::{STRIPE_WEBHOOK_TOLERANCE_SECONDS, to_workspace_id};
use crate::hex_encode;

#[path = "stripe.customer.rs"]
mod customer;
#[path = "stripe.http.rs"]
mod http;
#[path = "stripe.sessions.rs"]
mod sessions;

pub use customer::{build_customer_fields, create_stripe_customer};
pub use http::stripe_post_form;
pub use sessions::{
    build_checkout_session_fields, create_stripe_checkout_session, create_stripe_portal_session,
};

pub struct StripeSession {
    pub id: String,
    pub url: String,
}

pub struct StripeCustomer {
    pub id: String,
}

#[derive(Debug, Error)]
pub enum StripeProviderError {
    #[error("stripe_not_configured")]
    NotConfigured,
    #[error("stripe_live_key_rejected: Stripe live keys are forbidden in V1")]
    LiveKeyRejected,
    #[error("stripe_request_failed: {0}")]
    RequestFailed(String),
    #[error("stripe_response_failed: {0}")]
    ResponseFailed(String),
    #[error("stripe_response_invalid: {0}")]
    ResponseInvalid(String),
    #[error("stripe_request_rejected: {message}")]
    RequestRejected { status: u16, message: String },
}

impl StripeProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "stripe_not_configured",
            Self::LiveKeyRejected => "stripe_live_key_rejected",
            Self::RequestFailed(_) => "stripe_request_failed",
            Self::ResponseFailed(_) => "stripe_response_failed",
            Self::ResponseInvalid(_) => "stripe_response_invalid",
            Self::RequestRejected { .. } => "stripe_request_rejected",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::NotConfigured => {
                "NVBES_STRIPE_SECRET_KEY must be configured before billing actions.".to_string()
            }
            Self::LiveKeyRejected => {
                "Stripe live keys are forbidden in V1. Use test keys (sk_test_ / rk_test_).".to_string()
            }
            Self::RequestFailed(message)
            | Self::ResponseFailed(message)
            | Self::ResponseInvalid(message)
            | Self::RequestRejected { message, .. } => message.clone(),
        }
    }

    pub fn is_bad_request(&self) -> bool {
        matches!(self, Self::RequestRejected { status: 400, .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeWebhookEvent {
    pub id: String,
    pub event_type: String,
    pub livemode: bool,
    pub data_object: Value,
}

pub fn parse_stripe_signature_header(header: &str) -> Option<(String, Vec<String>)> {
    let mut timestamp = None;
    let mut signatures = Vec::new();

    for part in header.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "t" => timestamp = Some(value.to_owned()),
                "v1" => signatures.push(value.to_owned()),
                _ => {}
            }
        }
    }

    let timestamp = timestamp?;
    if signatures.is_empty() {
        return None;
    }

    Some((timestamp, signatures))
}

pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    let mut normalized_key = [0_u8; 64];
    if key.len() > 64 {
        let digest = Sha256::digest(key);
        normalized_key[..32].copy_from_slice(&digest);
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut outer = [0x5c_u8; 64];
    let mut inner = [0x36_u8; 64];
    for index in 0..64 {
        outer[index] ^= normalized_key[index];
        inner[index] ^= normalized_key[index];
    }

    let mut inner_hasher = Sha256::new();
    inner_hasher.update(inner);
    inner_hasher.update(message);
    let inner_digest = inner_hasher.finalize();

    let mut outer_hasher = Sha256::new();
    outer_hasher.update(outer);
    outer_hasher.update(inner_digest);
    hex_encode(&outer_hasher.finalize())
}

pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

pub fn parse_stripe_event(payload: &[u8]) -> Option<StripeWebhookEvent> {
    let value: Value = serde_json::from_slice(payload).ok()?;
    let id = value.get("id").and_then(Value::as_str)?.to_owned();
    let event_type = value.get("type").and_then(Value::as_str)?.to_owned();
    let livemode = value.get("livemode").and_then(Value::as_bool).unwrap_or(false);
    let data_object = value.pointer("/data/object").cloned()?;
    Some(StripeWebhookEvent {
        id,
        event_type,
        livemode,
        data_object,
    })
}

pub fn metadata_workspace_id(object: &Value) -> Option<Uuid> {
    object
        .pointer("/metadata/workspace_id")
        .and_then(Value::as_str)
        .and_then(to_workspace_id)
}

pub fn required_string(object: &Value, field: &str) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}

pub fn timestamp_field(object: &Value, field: &str) -> Option<DateTime<Utc>> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|timestamp| Utc.timestamp_opt(timestamp, 0).single())
}

pub fn stripe_subscription_status(status: Option<&str>) -> &'static str {
    match status {
        Some("active") => "active",
        Some("trialing") => "trialing",
        Some("past_due") => "past_due",
        Some("canceled") | Some("cancelled") => "canceled",
        Some("incomplete") | Some("incomplete_expired") => "incomplete",
        Some("unpaid") | Some("paused") => "suspended",
        _ => "suspended",
    }
}

pub fn verify_stripe_signature(
    webhook_secret: &str,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<(), &'static str> {
    let signature_header = signature_header.ok_or("missing_stripe_signature")?;
    let (timestamp, signatures) =
        parse_stripe_signature_header(signature_header).ok_or("invalid_stripe_signature")?;

    let timestamp_seconds = timestamp
        .parse::<i64>()
        .map_err(|_| "invalid_stripe_signature_timestamp")?;

    let age_seconds = Utc::now()
        .timestamp()
        .saturating_sub(timestamp_seconds)
        .abs();

    if age_seconds > STRIPE_WEBHOOK_TOLERANCE_SECONDS {
        return Err("stale_stripe_signature");
    }

    let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
    let expected = hmac_sha256_hex(webhook_secret.as_bytes(), signed_payload.as_bytes());

    if signatures
        .iter()
        .any(|signature| constant_time_eq(signature.as_bytes(), expected.as_bytes()))
    {
        Ok(())
    } else {
        Err("invalid_stripe_signature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_header(secret: &str, payload: &[u8], timestamp: i64) -> String {
        let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
        let signature = hmac_sha256_hex(secret.as_bytes(), signed_payload.as_bytes());
        format!("t={timestamp},v1={signature}")
    }

    #[test]
    fn verify_stripe_signature_accepts_valid_signature() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_1","type":"checkout.session.completed","data":{"object":{"id":"cs_1"}}}"#;
        let header = signed_header(secret, payload, Utc::now().timestamp());

        assert!(verify_stripe_signature(secret, Some(&header), payload).is_ok());
    }

    #[test]
    fn verify_stripe_signature_rejects_stale_signature() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_1","type":"checkout.session.completed","data":{"object":{"id":"cs_1"}}}"#;
        let header = signed_header(
            secret,
            payload,
            Utc::now().timestamp() - STRIPE_WEBHOOK_TOLERANCE_SECONDS - 1,
        );

        assert_eq!(
            verify_stripe_signature(secret, Some(&header), payload),
            Err("stale_stripe_signature")
        );
    }
}
