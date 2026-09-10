use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct ValidateTestEnv;

#[async_trait]
impl Keyword for ValidateTestEnv {
    async fn execute(
        &self,
        _ctx: &TestContext,
        _params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let env = std::env::var("NVBES_ENV").ok();
        crate::environment::validate_test_environment(env.as_deref())
            .map_err(KeywordError::Execution)?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        Ok(output)
    }
}

struct ValidateLoopbackUrl;

#[async_trait]
impl Keyword for ValidateLoopbackUrl {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = params.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
            KeywordError::InvalidParams("missing required param: url".to_string())
        })?;
        let resource = params
            .get("resource")
            .and_then(|v| v.as_str())
            .unwrap_or("URL");
        crate::environment::validate_loopback_url(url, resource)
            .map_err(KeywordError::Execution)?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::String("ok".to_string()));
        Ok(output)
    }
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("env.validate_test", Box::new(ValidateTestEnv));
    registry.register("env.validate_loopback", Box::new(ValidateLoopbackUrl));
}
