use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct HttpGet;

#[async_trait]
impl Keyword for HttpGet {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = require_string(params, "url")?;
        let client = crate::runner::transport::client()?;
        let mut req = client.get(url);
        if let Some(headers) = params.get("headers")
            && let Some(obj) = headers.as_object()
        {
            for (k, v) in obj {
                if let Some(val) = v.as_str() {
                    req = req.header(k.as_str(), val);
                }
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("HTTP GET failed: {e}")))?;
        let status = resp.status().as_u16();
        let headers: HashMap<String, String> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .text()
            .await
            .map_err(|e| KeywordError::Execution(format!("reading body: {e}")))?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        output.insert("body".to_string(), Value::String(body));
        output.insert(
            "headers".to_string(),
            serde_json::to_value(headers).unwrap_or(Value::Null),
        );
        Ok(output)
    }
}

struct HttpPost;

#[async_trait]
impl Keyword for HttpPost {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = require_string(params, "url")?;
        let client = crate::runner::transport::client()?;
        let mut req = client.post(url);
        if let Some(headers) = params.get("headers")
            && let Some(obj) = headers.as_object()
        {
            for (k, v) in obj {
                if let Some(val) = v.as_str() {
                    req = req.header(k.as_str(), val);
                }
            }
        }
        if let Some(body) = params.get("body") {
            let content_type = params
                .get("content_type")
                .and_then(|v| v.as_str())
                .unwrap_or("application/json");
            if content_type == "application/json" {
                req = req.json(body);
            } else if let Some(s) = body.as_str() {
                req = req.header("content-type", content_type).body(s.to_string());
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("HTTP POST failed: {e}")))?;
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| KeywordError::Execution(format!("reading body: {e}")))?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        output.insert("body".to_string(), Value::String(body));
        Ok(output)
    }
}

struct HttpAssertStatus;

#[async_trait]
impl Keyword for HttpAssertStatus {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = require_string(params, "url")?;
        let expected = params
            .get("expected_status")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        let client = crate::runner::transport::client()?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("HTTP request failed: {e}")))?;
        let actual = resp.status().as_u16();
        if actual != expected {
            return Err(KeywordError::Assertion(format!(
                "expected HTTP {expected} from {url}, got {actual}"
            )));
        }
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        Ok(output)
    }
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
    registry.register("http.get", Box::new(HttpGet));
    registry.register("http.post", Box::new(HttpPost));
    registry.register("http.assert_status", Box::new(HttpAssertStatus));
}
