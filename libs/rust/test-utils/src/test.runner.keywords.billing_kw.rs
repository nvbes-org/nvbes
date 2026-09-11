use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde_json::{Map, Value, json};
use sha2::Sha256;
use uuid::Uuid;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

const STRIPE_TOLERANCE_SECONDS: u64 = 300;
const DEFAULT_WEBHOOK_SECRET: &str = "whsec_test_secret";

type HmacSha256 = Hmac<Sha256>;

struct BillingCreateWebhook;

#[async_trait]
impl Keyword for BillingCreateWebhook {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let event_type = require_string(params, "event_type")?;
        let payload = params
            .get("payload")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        let livemode = params
            .get("livemode")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let secret = webhook_secret(ctx, params)?;
        let timestamp = now_seconds();
        let event = json!({
            "id": format!("evt_test_{}", Uuid::new_v4()),
            "type": event_type,
            "livemode": livemode,
            "data": { "object": payload },
        });
        let event_payload = event.to_string();
        let signature = format!(
            "t={timestamp},v1={}",
            sign(&secret, timestamp, &event_payload)
        );

        let mut output = HashMap::new();
        output.insert("event_id".to_string(), event.get("id").unwrap().clone());
        output.insert("event".to_string(), event);
        output.insert("event_payload".to_string(), Value::String(event_payload));
        output.insert("signature".to_string(), Value::String(signature));
        output.insert("timestamp".to_string(), Value::Number(timestamp.into()));
        Ok(output)
    }
}

struct BillingVerifySignature;

#[async_trait]
impl Keyword for BillingVerifySignature {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let payload = require_string(params, "payload")?;
        let signature = require_string(params, "signature")?;
        let secret = require_string(params, "secret")?;
        let parsed = parse_signature_header(signature).ok_or_else(|| {
            KeywordError::InvalidParams("invalid stripe signature header".to_string())
        })?;
        let now = now_seconds();
        let stale = now.abs_diff(parsed.timestamp) > STRIPE_TOLERANCE_SECONDS;
        let expected = sign(secret, parsed.timestamp, payload);
        let valid = !stale
            && parsed
                .signatures
                .iter()
                .any(|candidate| constant_time_eq(candidate.as_bytes(), expected.as_bytes()));

        let mut output = HashMap::new();
        output.insert("valid".to_string(), Value::Bool(valid));
        output.insert("stale".to_string(), Value::Bool(stale));
        output.insert("expected_signature".to_string(), Value::String(expected));
        Ok(output)
    }
}

struct BillingProcessEvent;

#[async_trait]
impl Keyword for BillingProcessEvent {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        deliver_signed_webhook(ctx, params, 1).await
    }
}

struct BillingCheckIdempotence;

#[async_trait]
impl Keyword for BillingCheckIdempotence {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        deliver_signed_webhook(ctx, params, 2).await
    }
}

async fn deliver_signed_webhook(
    ctx: &TestContext,
    params: &HashMap<String, Value>,
    deliveries: u64,
) -> Result<HashMap<String, Value>, KeywordError> {
    let payload = webhook_payload(params)?;
    let header = signature_header(ctx, params, &payload)?;
    let base_url = billing_base_url(ctx);
    let client = crate::runner::transport::client()?;
    let mut output = HashMap::new();
    let mut handled = true;
    for delivery in 1..=deliveries {
        let resp = client
            .post(format!("{base_url}/webhooks/stripe"))
            .header("Stripe-Signature", header.as_str())
            .header("Content-Type", "application/json")
            .body(payload.clone())
            .send()
            .await
            .map_err(|e| {
                KeywordError::Execution(format!("billing webhook delivery failed: {e}"))
            })?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        output.insert(
            format!("delivery_{delivery}_status"),
            Value::Number(status.into()),
        );
        if !(200..300).contains(&status) {
            handled = false;
        }
        if let Some(err) = body.get("error") {
            output.insert("error_message".to_string(), err.clone());
        }
        if let Some(processed) = body.get("processed") {
            output.insert("processed".to_string(), processed.clone());
        }
    }
    output.insert("idempotent_handled".to_string(), Value::Bool(handled));
    Ok(output)
}

fn webhook_payload(params: &HashMap<String, Value>) -> Result<String, KeywordError> {
    if let Some(payload) = params.get("event_payload").and_then(Value::as_str) {
        return Ok(payload.to_string());
    }
    let event = params.get("event").ok_or_else(|| {
        KeywordError::InvalidParams("missing required param: event or event_payload".to_string())
    })?;
    Ok(event.to_string())
}

fn signature_header(
    ctx: &TestContext,
    params: &HashMap<String, Value>,
    payload: &str,
) -> Result<String, KeywordError> {
    if let Some(signature) = params.get("signature").and_then(Value::as_str) {
        return Ok(signature.to_string());
    }
    let secret = webhook_secret(ctx, params)?;
    let timestamp = now_seconds();
    Ok(format!(
        "t={timestamp},v1={}",
        sign(&secret, timestamp, payload)
    ))
}

fn sign(secret: &str, timestamp: u64, payload: &str) -> String {
    let signed_payload = format!("{timestamp}.{payload}");
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(signed_payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

struct HeaderSignature {
    timestamp: u64,
    signatures: Vec<String>,
}

fn parse_signature_header(header: &str) -> Option<HeaderSignature> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        let (key, value) = part.split_once('=')?;
        match key {
            "t" => timestamp = Some(value),
            "v1" => signatures.push(value.to_string()),
            _ => {}
        }
    }
    Some(HeaderSignature {
        timestamp: timestamp?.parse().ok()?,
        signatures,
    })
}

fn webhook_secret(
    ctx: &TestContext,
    params: &HashMap<String, Value>,
) -> Result<String, KeywordError> {
    if let Some(secret) = params.get("secret").and_then(Value::as_str) {
        return Ok(secret.to_string());
    }
    Ok(ctx
        .variables
        .env
        .get("NVBES_BILLING_WEBHOOK_SECRET")
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_WEBHOOK_SECRET)
        .to_string())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

fn billing_base_url(ctx: &TestContext) -> &str {
    ctx.variables
        .env
        .get("NVBES_BILLING_BASE_URL")
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:3002")
}

fn require_string<'a>(
    params: &'a HashMap<String, Value>,
    key: &str,
) -> Result<&'a str, KeywordError> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| KeywordError::InvalidParams(format!("missing required param: {key}")))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for i in 0..a.len() {
        result |= a[i] ^ b[i];
    }
    result == 0
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("billing.create_webhook", Box::new(BillingCreateWebhook));
    registry.register("billing.verify_signature", Box::new(BillingVerifySignature));
    registry.register("billing.process_event", Box::new(BillingProcessEvent));
    registry.register(
        "billing.check_idempotence",
        Box::new(BillingCheckIdempotence),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::variables::VariableContext;

    fn ctx() -> TestContext {
        TestContext {
            variables: VariableContext::new(HashMap::new()),
            resources: Default::default(),
        }
    }

    fn signed_header(secret: &str, payload: &str, timestamp: u64) -> String {
        format!("t={timestamp},v1={}", sign(secret, timestamp, payload))
    }

    #[tokio::test]
    async fn verify_signature_accepts_fresh_header() {
        let secret = DEFAULT_WEBHOOK_SECRET;
        let payload = r#"{"id":"evt_test_1","type":"checkout.session.completed"}"#;
        let header = signed_header(secret, payload, now_seconds());
        let mut params = HashMap::new();
        params.insert("payload".to_string(), Value::String(payload.to_string()));
        params.insert("signature".to_string(), Value::String(header));
        params.insert("secret".to_string(), Value::String(secret.to_string()));
        let result = BillingVerifySignature
            .execute(&ctx(), &params)
            .await
            .unwrap();
        assert_eq!(result.get("valid").unwrap(), true);
        assert_eq!(result.get("stale").unwrap(), false);
    }

    #[tokio::test]
    async fn verify_signature_rejects_wrong_secret() {
        let secret = DEFAULT_WEBHOOK_SECRET;
        let payload = r#"{"id":"evt_test_1","type":"checkout.session.completed"}"#;
        let header = signed_header(secret, payload, now_seconds());
        let mut params = HashMap::new();
        params.insert("payload".to_string(), Value::String(payload.to_string()));
        params.insert("signature".to_string(), Value::String(header));
        params.insert(
            "secret".to_string(),
            Value::String("other-secret".to_string()),
        );
        let result = BillingVerifySignature
            .execute(&ctx(), &params)
            .await
            .unwrap();
        assert_eq!(result.get("valid").unwrap(), false);
    }

    #[tokio::test]
    async fn verify_signature_rejects_stale_header() {
        let secret = DEFAULT_WEBHOOK_SECRET;
        let payload = r#"{"id":"evt_test_1"}"#;
        let header = signed_header(
            secret,
            payload,
            now_seconds() - STRIPE_TOLERANCE_SECONDS - 1,
        );
        let mut params = HashMap::new();
        params.insert("payload".to_string(), Value::String(payload.to_string()));
        params.insert("signature".to_string(), Value::String(header));
        params.insert("secret".to_string(), Value::String(secret.to_string()));
        let result = BillingVerifySignature
            .execute(&ctx(), &params)
            .await
            .unwrap();
        assert_eq!(result.get("valid").unwrap(), false);
        assert_eq!(result.get("stale").unwrap(), true);
    }

    #[tokio::test]
    async fn verify_signature_rejects_malformed_header() {
        let mut params = HashMap::new();
        params.insert("payload".to_string(), Value::String("{}".to_string()));
        params.insert(
            "signature".to_string(),
            Value::String("not-a-header".to_string()),
        );
        params.insert("secret".to_string(), Value::String("secret".to_string()));
        let result = BillingVerifySignature.execute(&ctx(), &params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_webhook_builds_signed_event() {
        let mut params = HashMap::new();
        params.insert(
            "event_type".to_string(),
            Value::String("invoice.paid".to_string()),
        );
        let result = BillingCreateWebhook.execute(&ctx(), &params).await.unwrap();
        let signature = result.get("signature").unwrap().as_str().unwrap();
        let timestamp = result.get("timestamp").unwrap().as_u64().unwrap();
        assert!(
            result
                .get("event_payload")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("invoice.paid")
        );
        assert!(signature.starts_with("t="));
        assert!((now_seconds() - timestamp) < 2);
    }
}
