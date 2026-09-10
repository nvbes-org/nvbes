use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct EmailSend;

#[async_trait]
impl Keyword for EmailSend {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let message_id = require_string(params, "message_id")?;
        deliver_dispatch(ctx, message_id, "send").await
    }
}

struct EmailRetryFailed;

#[async_trait]
impl Keyword for EmailRetryFailed {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let message_id = require_string(params, "message_id")?;
        deliver_dispatch(ctx, message_id, "retry_failed").await
    }
}

async fn deliver_dispatch(
    ctx: &TestContext,
    message_id: &str,
    result_key: &str,
) -> Result<HashMap<String, Value>, KeywordError> {
    let base_url = email_base_url(ctx);
    let client = crate::runner::transport::client()?;
    let mut request = client
        .post(format!("{base_url}/internal/queue/email-dispatch"))
        .body(message_id.to_string());
    if let Some(token) = internal_token(ctx) {
        request = request.header("Authorization", token);
    }
    let resp = request
        .send()
        .await
        .map_err(|e| KeywordError::Execution(format!("email dispatch request failed: {e}")))?;
    let status = resp.status().as_u16();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    let mut output = HashMap::new();
    output.insert("status".to_string(), Value::Number(status.into()));
    output.insert(result_key.to_string(), Value::Bool(status == 204));
    if !body.is_null() {
        output.insert("body".to_string(), body);
    }
    Ok(output)
}

struct EmailCaptureWebhook;

#[async_trait]
impl Keyword for EmailCaptureWebhook {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let message = require_string(params, "message")?;
        let message_type = params
            .get("message_type")
            .and_then(Value::as_str)
            .unwrap_or("Notification");
        let topic = params
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or("arn:scw:sns:fr-par:test:topic");
        let payload = json!({
            "Type": message_type,
            "MessageId": format!("sns-{}", uuid::Uuid::new_v4()),
            "TopicArn": topic,
            "Message": message,
            "Timestamp": chrono::Utc::now().to_rfc3339(),
            "SignatureVersion": "1",
            "Signature": "keyword-generated",
            "SigningCertURL": "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem",
        });
        let base_url = email_base_url(ctx);
        let client = crate::runner::transport::client()?;
        let mut request = client
            .post(format!("{base_url}/webhooks/scaleway/topics-and-events"))
            .header("Content-Type", "application/json")
            .json(&payload);
        if let Some(token) = internal_token(ctx) {
            request = request.header("Authorization", token);
        }
        let resp = request
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("email webhook request failed: {e}")))?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        if let Some(kind) = body.get("kind") {
            output.insert("kind".to_string(), kind.clone());
        }
        Ok(output)
    }
}

struct EmailVerifyDelivery;

#[async_trait]
impl Keyword for EmailVerifyDelivery {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let expected = params
            .get("expected_messages")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(10_000);
        let interval_ms = params
            .get("interval_ms")
            .and_then(Value::as_u64)
            .unwrap_or(500);
        let base_url = email_base_url(ctx);
        let client = crate::runner::transport::client()?;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
        let delivered;
        let mut messages_total;
        let mut webhooks_total;
        loop {
            let mut request = client.get(format!("{base_url}/metrics"));
            if let Some(metrics_token) = metrics_token(ctx) {
                request = request.header("Authorization", metrics_token);
            }
            let resp = request.send().await.map_err(|e| {
                KeywordError::Execution(format!("email metrics request failed: {e}"))
            })?;
            let text = resp
                .text()
                .await
                .map_err(|e| KeywordError::Execution(format!("email metrics body failed: {e}")))?;
            messages_total = metric_value(&text, "email_messages_total");
            webhooks_total = metric_value(&text, "email_provider_webhooks_total");
            if messages_total >= expected || tokio::time::Instant::now() >= deadline {
                delivered = messages_total >= expected;
                break;
            }
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
        let mut output = HashMap::new();
        output.insert("delivered".to_string(), Value::Bool(delivered));
        output.insert(
            "messages_total".to_string(),
            Value::Number(messages_total.into()),
        );
        output.insert(
            "webhooks_total".to_string(),
            Value::Number(webhooks_total.into()),
        );
        Ok(output)
    }
}

fn metric_value(text: &str, name: &str) -> u64 {
    text.lines()
        .filter(|line| {
            line.starts_with(&format!("{name}{{")) || line.starts_with(&format!("{name} "))
        })
        .filter_map(|line| {
            line.split_whitespace()
                .last()
                .and_then(|v| v.parse::<u64>().ok())
        })
        .sum()
}

fn email_base_url(ctx: &TestContext) -> &str {
    ctx.variables
        .env
        .get("NVBES_EMAIL_BASE_URL")
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:3040")
}

fn internal_token(ctx: &TestContext) -> Option<String> {
    ctx.variables.env.get("NVBES_EMAIL_INTERNAL_TOKEN").cloned()
}

fn metrics_token(ctx: &TestContext) -> Option<String> {
    ctx.variables
        .env
        .get("NVBES_EMAIL_METRICS_TOKEN")
        .map(|token| format!("Bearer {token}"))
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

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("email.send", Box::new(EmailSend));
    registry.register("email.retry_failed", Box::new(EmailRetryFailed));
    registry.register("email.capture_webhook", Box::new(EmailCaptureWebhook));
    registry.register("email.verify_delivery", Box::new(EmailVerifyDelivery));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_counter_metric_with_labels() {
        let text = "# HELP email_messages_total Accepted messages\n\
            # TYPE email_messages_total counter\n\
            email_messages_total{kind=\"user\"} 4\n\
            email_messages_total{kind=\"transactional\"} 2\n";
        assert_eq!(metric_value(text, "email_messages_total"), 6);
        assert_eq!(metric_value(text, "email_provider_webhooks_total"), 0);
    }

    #[test]
    fn parses_named_counter_metric() {
        let text = "email_messages_total 17\nemail_provider_webhooks_total 3\n";
        assert_eq!(metric_value(text, "email_messages_total"), 17);
        assert_eq!(metric_value(text, "email_provider_webhooks_total"), 3);
    }

    #[test]
    fn parses_missing_metric_as_zero() {
        assert_eq!(
            metric_value("no relevant lines here\n", "email_messages_total"),
            0
        );
    }
}
