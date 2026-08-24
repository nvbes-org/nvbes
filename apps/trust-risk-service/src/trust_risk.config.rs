use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use serde::Deserialize;

const DEVELOPMENT_TOKEN: &str = "development-trust-risk-token-32-value";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProducerPolicy {
    pub producer: String,
    pub token: String,
    pub signal_prefixes: Vec<String>,
    #[serde(default)]
    pub can_assess: bool,
    #[serde(default)]
    pub can_label: bool,
}

impl ProducerPolicy {
    pub fn permits_signal(&self, kind: &str) -> bool {
        self.signal_prefixes
            .iter()
            .any(|prefix| kind.starts_with(prefix))
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorPolicy {
    pub actor: String,
    pub token: String,
    pub permissions: Vec<String>,
}

impl OperatorPolicy {
    pub fn permits(&self, permission: &str) -> bool {
        self.permissions.iter().any(|value| value == permission)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionConfig {
    pub signals_days: u32,
    pub evaluations_days: u32,
    pub labels_days: u32,
    pub reviews_days: u32,
    pub audit_days: u32,
}

#[derive(Debug, Clone)]
pub struct TrustRiskConfig {
    pub environment: String,
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub producers: BTreeMap<String, ProducerPolicy>,
    pub operators: BTreeMap<String, OperatorPolicy>,
    pub metrics_token: String,
    pub retention: RetentionConfig,
    pub projection_heartbeat_max_age: Duration,
}

impl TrustRiskConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = env_or("NVBES_ENVIRONMENT", "development");
        let development = matches!(environment.as_str(), "development" | "test");
        let database_url = database_url(&environment)?;
        let bind_addr = env_or("NVBES_TRUST_RISK_BIND_ADDR", "127.0.0.1:3050")
            .parse()
            .map_err(|_| ConfigError::Invalid("NVBES_TRUST_RISK_BIND_ADDR"))?;
        let producer_json = optional("NVBES_TRUST_RISK_PRODUCER_POLICIES").or_else(|| {
            development.then(|| {
                format!(
                    r#"[{{"producer":"billing-checkout-fixture","token":"{DEVELOPMENT_TOKEN}","signal_prefixes":["network.","payment.","velocity."],"can_assess":true,"can_label":true}}]"#
                )
            })
        });
        let operator_json = optional("NVBES_TRUST_RISK_OPERATOR_TOKENS").or_else(|| {
            development.then(|| {
                format!(
                    r#"[{{"actor":"development-operator","token":"{DEVELOPMENT_TOKEN}","permissions":["evaluation:read","review:write","rules:write","labels:human"]}}]"#
                )
            })
        });
        let producers = parse_producers(
            producer_json
                .as_deref()
                .ok_or(ConfigError::Missing("NVBES_TRUST_RISK_PRODUCER_POLICIES"))?,
        )?;
        let operators = parse_operators(
            operator_json
                .as_deref()
                .ok_or(ConfigError::Missing("NVBES_TRUST_RISK_OPERATOR_TOKENS"))?,
        )?;
        let retention = retention_from_env(development)?;
        let metrics_token = optional("NVBES_TRUST_RISK_METRICS_TOKEN")
            .or_else(|| development.then(|| DEVELOPMENT_TOKEN.to_string()))
            .ok_or(ConfigError::Missing("NVBES_TRUST_RISK_METRICS_TOKEN"))?;
        if metrics_token.len() < 32 || metrics_token.contains(['\r', '\n']) {
            return Err(ConfigError::WeakToken);
        }

        Ok(Self {
            environment,
            database_url,
            bind_addr,
            producers,
            operators,
            metrics_token,
            retention,
            projection_heartbeat_max_age: Duration::from_secs(30),
        })
    }
}

pub fn database_url_from_env() -> Result<String, ConfigError> {
    let environment = env_or("NVBES_ENVIRONMENT", "development");
    database_url(&environment)
}

fn database_url(environment: &str) -> Result<String, ConfigError> {
    let development = matches!(environment, "development" | "test");
    optional("NVBES_TRUST_RISK_DATABASE_URL")
        .or_else(|| development.then(|| "postgres://localhost/nvbes_trust_risk".to_string()))
        .ok_or(ConfigError::Missing("NVBES_TRUST_RISK_DATABASE_URL"))
}

pub fn parse_producers(value: &str) -> Result<BTreeMap<String, ProducerPolicy>, ConfigError> {
    let policies: Vec<ProducerPolicy> = serde_json::from_str(value)
        .map_err(|_| ConfigError::Invalid("NVBES_TRUST_RISK_PRODUCER_POLICIES"))?;
    collect_policies(policies, |policy| &policy.producer)
}

pub fn parse_operators(value: &str) -> Result<BTreeMap<String, OperatorPolicy>, ConfigError> {
    let policies: Vec<OperatorPolicy> = serde_json::from_str(value)
        .map_err(|_| ConfigError::Invalid("NVBES_TRUST_RISK_OPERATOR_TOKENS"))?;
    collect_policies(policies, |policy| &policy.actor)
}

fn collect_policies<T: HasToken>(
    policies: Vec<T>,
    key: impl Fn(&T) -> &str,
) -> Result<BTreeMap<String, T>, ConfigError> {
    if policies.is_empty() {
        return Err(ConfigError::Invalid("policy list"));
    }
    let mut result = BTreeMap::new();
    for policy in policies {
        validate_identifier(key(&policy))?;
        if policy.token().len() < 32 || policy.token().contains(['\r', '\n']) {
            return Err(ConfigError::WeakToken);
        }
        let name = key(&policy).to_string();
        if result.insert(name, policy).is_some() {
            return Err(ConfigError::DuplicatePolicy);
        }
    }
    Ok(result)
}

trait HasToken {
    fn token(&self) -> &str;
}

impl HasToken for ProducerPolicy {
    fn token(&self) -> &str {
        &self.token
    }
}

impl HasToken for OperatorPolicy {
    fn token(&self) -> &str {
        &self.token
    }
}

fn validate_identifier(value: &str) -> Result<(), ConfigError> {
    let valid = (3..=80).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'));
    valid
        .then_some(())
        .ok_or(ConfigError::Invalid("policy identifier"))
}

fn retention_from_env(development: bool) -> Result<RetentionConfig, ConfigError> {
    Ok(RetentionConfig {
        signals_days: retention("NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS", development, 30)?,
        evaluations_days: retention(
            "NVBES_TRUST_RISK_EVALUATIONS_RETENTION_DAYS",
            development,
            400,
        )?,
        labels_days: retention("NVBES_TRUST_RISK_LABELS_RETENTION_DAYS", development, 400)?,
        reviews_days: retention("NVBES_TRUST_RISK_REVIEWS_RETENTION_DAYS", development, 400)?,
        audit_days: retention("NVBES_TRUST_RISK_AUDIT_RETENTION_DAYS", development, 730)?,
    })
}

fn retention(name: &'static str, development: bool, default: u32) -> Result<u32, ConfigError> {
    let value = optional(name)
        .or_else(|| development.then(|| default.to_string()))
        .ok_or(ConfigError::Missing(name))?;
    value
        .parse::<u32>()
        .ok()
        .filter(|days| (1..=3650).contains(days))
        .ok_or(ConfigError::Invalid(name))
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_or(name: &str, default: &str) -> String {
    optional(name).unwrap_or_else(|| default.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("required configuration is missing: {0}")]
    Missing(&'static str),
    #[error("configuration is invalid: {0}")]
    Invalid(&'static str),
    #[error("policy authentication token must contain at least 32 safe characters")]
    WeakToken,
    #[error("policy identifier is duplicated")]
    DuplicatePolicy,
}

#[cfg(test)]
#[path = "trust_risk.config.tests.rs"]
mod tests;
