use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct IdentityHealthCheck;

#[async_trait]
impl Keyword for IdentityHealthCheck {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let expected_status = params
            .get("expected_status")
            .and_then(Value::as_u64)
            .unwrap_or(200);
        let base_url = identity_base_url(ctx);
        let client = crate::runner::transport::client()?;
        let resp = client
            .get(format!("{base_url}/health/live"))
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("health check failed: {e}")))?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        output.insert(
            "healthy".to_string(),
            Value::Bool(status == expected_status as u16),
        );
        if !body.is_null() {
            output.insert("body".to_string(), body);
        }
        Ok(output)
    }
}

fn identity_base_url(ctx: &TestContext) -> &str {
    ctx.variables
        .env
        .get("NVBES_IDENTITY_BASE_URL")
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:3060")
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("identity.health_check", Box::new(IdentityHealthCheck));
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

    #[tokio::test]
    async fn health_check_defaults_to_live_endpoint() {
        // Default base URL must target the identity-service bind address, and no
        // public auth surface may be assumed: the container contract guarantees
        // /api/auth/* is absent, so only /health/live is probed.
        assert_eq!(
            identity_base_url(&ctx()),
            "http://127.0.0.1:3060",
            "identity-service bind address"
        );
    }
}
