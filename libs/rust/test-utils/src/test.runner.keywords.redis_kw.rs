use std::collections::HashMap;

use async_trait::async_trait;
use redis::AsyncCommands;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct HealthCheck;

#[async_trait]
impl Keyword for HealthCheck {
    async fn execute(
        &self,
        _ctx: &TestContext,
        _params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let pool = crate::redis::test_redis_pool()
            .await
            .ok_or_else(|| KeywordError::Execution("Redis is not reachable".to_string()))?;
        nvbes_redis::connection::health_check(&pool)
            .await
            .map_err(|e| KeywordError::Execution(format!("health check failed: {e}")))?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        Ok(output)
    }
}

struct SetValue;

#[async_trait]
impl Keyword for SetValue {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let key = require_string(params, "key")?;
        let value = require_string(params, "value")?;
        let pool = crate::redis::test_redis_pool()
            .await
            .ok_or_else(|| KeywordError::Execution("Redis is not reachable".to_string()))?;
        let mut conn = pool
            .get()
            .await
            .map_err(|e| KeywordError::Execution(format!("redis connection: {e}")))?;
        if let Some(ttl) = params.get("ttl").and_then(|v| v.as_u64()) {
            let _: () = conn
                .set_ex(key, value, ttl)
                .await
                .map_err(|e| KeywordError::Execution(format!("setex: {e}")))?;
        } else {
            let _: () = conn
                .set(key, value)
                .await
                .map_err(|e| KeywordError::Execution(format!("set: {e}")))?;
        }
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        output.insert("key".to_string(), Value::String(key.to_string()));
        Ok(output)
    }
}

struct GetValue;

#[async_trait]
impl Keyword for GetValue {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let key = require_string(params, "key")?;
        let pool = crate::redis::test_redis_pool()
            .await
            .ok_or_else(|| KeywordError::Execution("Redis is not reachable".to_string()))?;
        let mut conn = pool
            .get()
            .await
            .map_err(|e| KeywordError::Execution(format!("redis connection: {e}")))?;
        let result: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| KeywordError::Execution(format!("get: {e}")))?;
        let mut output = HashMap::new();
        match result {
            Some(val) => {
                output.insert("found".to_string(), Value::Bool(true));
                output.insert("value".to_string(), Value::String(val));
            }
            None => {
                output.insert("found".to_string(), Value::Bool(false));
                output.insert("value".to_string(), Value::Null);
            }
        }
        Ok(output)
    }
}

struct DelKey;

#[async_trait]
impl Keyword for DelKey {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let key = require_string(params, "key")?;
        let pool = crate::redis::test_redis_pool()
            .await
            .ok_or_else(|| KeywordError::Execution("Redis is not reachable".to_string()))?;
        let mut conn = pool
            .get()
            .await
            .map_err(|e| KeywordError::Execution(format!("redis connection: {e}")))?;
        let deleted: i64 = conn
            .del(key)
            .await
            .map_err(|e| KeywordError::Execution(format!("del: {e}")))?;
        let mut output = HashMap::new();
        output.insert("deleted".to_string(), Value::Number(deleted.into()));
        Ok(output)
    }
}

struct PublishMessage;

#[async_trait]
impl Keyword for PublishMessage {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let channel = require_string(params, "channel")?;
        let message = require_string(params, "message")?;
        let pool = crate::redis::test_redis_pool()
            .await
            .ok_or_else(|| KeywordError::Execution("Redis is not reachable".to_string()))?;
        let mut conn = pool
            .get()
            .await
            .map_err(|e| KeywordError::Execution(format!("redis connection: {e}")))?;
        let receivers: i64 = conn
            .publish(channel, message)
            .await
            .map_err(|e| KeywordError::Execution(format!("publish: {e}")))?;
        let mut output = HashMap::new();
        output.insert("receivers".to_string(), Value::Number(receivers.into()));
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
    registry.register("redis.health_check", Box::new(HealthCheck));
    registry.register("redis.set_value", Box::new(SetValue));
    registry.register("redis.get_value", Box::new(GetValue));
    registry.register("redis.del_key", Box::new(DelKey));
    registry.register("redis.publish", Box::new(PublishMessage));
}
