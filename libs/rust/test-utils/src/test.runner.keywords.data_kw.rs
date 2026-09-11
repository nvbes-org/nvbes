use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct SetVar;

#[async_trait]
impl Keyword for SetVar {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let name = require_string(params, "name")?;
        let value = params.get("value").cloned().unwrap_or(Value::Null);
        let mut output = HashMap::new();
        output.insert("name".to_string(), Value::String(name.to_string()));
        output.insert("value".to_string(), value);
        Ok(output)
    }
}

struct GetVar;

#[async_trait]
impl Keyword for GetVar {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let name = require_string(params, "name")?;
        let value = ctx
            .variables
            .env
            .get(name)
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Null);
        let mut output = HashMap::new();
        output.insert("value".to_string(), value);
        Ok(output)
    }
}

struct AssertEq;

#[async_trait]
impl Keyword for AssertEq {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let actual = params.get("actual").ok_or_else(|| {
            KeywordError::InvalidParams("missing required param: actual".to_string())
        })?;
        let expected = params.get("expected").ok_or_else(|| {
            KeywordError::InvalidParams("missing required param: expected".to_string())
        })?;
        if actual != expected {
            return Err(KeywordError::Assertion(format!(
                "assert_eq failed: {actual} != {expected}"
            )));
        }
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        Ok(output)
    }
}

struct Timestamp;

#[async_trait]
impl Keyword for Timestamp {
    async fn execute(
        &self,
        _ctx: &TestContext,
        _params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut output = HashMap::new();
        output.insert("timestamp".to_string(), Value::String(now));
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
    registry.register("data.set_var", Box::new(SetVar));
    registry.register("data.get_var", Box::new(GetVar));
    registry.register("data.assert_eq", Box::new(AssertEq));
    registry.register("data.timestamp", Box::new(Timestamp));
}
