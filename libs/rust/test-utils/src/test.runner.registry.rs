use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use super::TestContext;

#[derive(Debug, thiserror::Error)]
pub enum KeywordError {
    #[error("keyword not found: {0}")]
    NotFound(String),

    #[error("invalid parameters: {0}")]
    InvalidParams(String),

    #[error("execution failed: {0}")]
    Execution(String),

    #[error("assertion failed: {0}")]
    Assertion(String),
}

#[async_trait]
pub trait Keyword: Send + Sync {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError>;
}

#[derive(Default)]
pub struct KeywordRegistry {
    keywords: HashMap<String, Box<dyn Keyword>>,
}

impl KeywordRegistry {
    pub fn new() -> Self {
        Self {
            keywords: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, keyword: Box<dyn Keyword>) {
        self.keywords.insert(name.into(), keyword);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Keyword> {
        self.keywords.get(name).map(|k| k.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.keywords.contains_key(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.keywords.keys().map(|s| s.as_str()).collect()
    }
}

pub fn register_all(registry: &mut KeywordRegistry) {
    super::keywords::redis_kw::register(registry);
    super::keywords::env_kw::register(registry);
    super::keywords::data_kw::register(registry);
    super::keywords::http_kw::register(registry);
    super::keywords::identity_kw::register(registry);
    super::keywords::platform_kw::register(registry);
    super::keywords::billing_kw::register(registry);
    super::keywords::email_kw::register(registry);
    super::keywords::infra_kw::register(registry);
    super::keywords::trust_risk_kw::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_registry() -> KeywordRegistry {
        let mut registry = KeywordRegistry::new();
        register_all(&mut registry);
        registry
    }

    #[test]
    fn all_domains_registered() {
        let expected = [
            "billing.create_webhook",
            "billing.verify_signature",
            "billing.process_event",
            "billing.check_idempotence",
            "email.send",
            "email.verify_delivery",
            "email.capture_webhook",
            "email.retry_failed",
            "infra.setup_db",
            "infra.migrate",
            "infra.start_service",
            "infra.health_check",
            "infra.cleanup",
            "trust.submit_signals",
            "trust.assess_risk",
            "trust.submit_labels",
            "identity.health_check",
            "http.get",
            "http.post",
            "http.assert_status",
            "redis.health_check",
            "redis.set_value",
            "redis.get_value",
            "redis.del_key",
            "redis.publish",
            "env.validate_test",
            "env.validate_loopback",
            "data.set_var",
            "data.get_var",
            "data.assert_eq",
            "data.timestamp",
            "platform.health_probe",
            "platform.cockpit_overview",
            "platform.budget_stage",
            "platform.degraded_procedure",
            "platform.service_readiness",
            "platform.wait_for_ready",
        ];
        let registry = full_registry();
        for name in expected {
            assert!(
                registry.has(name),
                "keyword {name} is missing from registry"
            );
        }
        assert_eq!(registry.list().len(), expected.len());
    }
}
