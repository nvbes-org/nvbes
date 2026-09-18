use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use nvbes_trust_risk::client::{TrustRiskClient, TrustRiskClientConfig, TrustRiskClientError};
use nvbes_trust_risk::proto::nvbes::platform::v1::{RequestContext, TenantContext};
use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
    AssessRiskRequest, AttributeValue, DataScope, LabelSourceClass, RiskBand, RiskEvaluation,
    RiskLabel, RiskLabelKind, RiskReason, RiskRecommendation, RiskSignal, SubjectKind,
    SubjectReference, SubmitLabelsRequest, SubmitSignalsRequest, attribute_value,
};
use prost_types::Timestamp;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

const DEFAULT_TIMEOUT_MS: u64 = 3000;
const DEFAULT_GRPC_ENDPOINT: &str = "http://127.0.0.1:3050";
const DEFAULT_SCOPE: i32 = DataScope::Tenant as i32;

struct TrustSubmitSignals;

#[async_trait]
impl Keyword for TrustSubmitSignals {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let signals = parse_signals(params)?;
        let client = connect(ctx, params).await?;
        let response = client
            .submit_signals(SubmitSignalsRequest { signals })
            .await
            .map_err(map_client_error)?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(200.into()));
        output.insert(
            "accepted".to_string(),
            Value::Number(
                response
                    .receipts
                    .iter()
                    .filter(|r| !r.duplicate)
                    .count()
                    .into(),
            ),
        );
        output.insert(
            "duplicates".to_string(),
            Value::Number(
                response
                    .receipts
                    .iter()
                    .filter(|r| r.duplicate)
                    .count()
                    .into(),
            ),
        );
        output.insert(
            "receipts".to_string(),
            Value::Array(
                response
                    .receipts
                    .iter()
                    .map(|receipt| {
                        json!({
                            "signal_id": receipt.signal_id,
                            "accepted_at": timestamp_rfc3339(receipt.accepted_at),
                            "duplicate": receipt.duplicate,
                        })
                    })
                    .collect(),
            ),
        );
        Ok(output)
    }
}

struct TrustAssessRisk;

#[async_trait]
impl Keyword for TrustAssessRisk {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let client = connect(ctx, params).await?;
        let request = parse_assessment(params)?;
        let evaluation = client
            .assess_risk(request)
            .await
            .map_err(map_client_error)?;
        Ok(evaluation_output(evaluation))
    }
}

struct TrustSubmitLabels;

#[async_trait]
impl Keyword for TrustSubmitLabels {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let labels = parse_labels(params)?;
        let client = connect(ctx, params).await?;
        let response = client
            .submit_labels(SubmitLabelsRequest { labels })
            .await
            .map_err(map_client_error)?;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(200.into()));
        output.insert(
            "accepted".to_string(),
            Value::Number(
                response
                    .receipts
                    .iter()
                    .filter(|r| !r.duplicate)
                    .count()
                    .into(),
            ),
        );
        output.insert(
            "duplicates".to_string(),
            Value::Number(
                response
                    .receipts
                    .iter()
                    .filter(|r| r.duplicate)
                    .count()
                    .into(),
            ),
        );
        output.insert(
            "receipts".to_string(),
            Value::Array(
                response
                    .receipts
                    .iter()
                    .map(|receipt| {
                        json!({
                            "label_id": receipt.label_id,
                            "accepted_at": timestamp_rfc3339(receipt.accepted_at),
                            "duplicate": receipt.duplicate,
                        })
                    })
                    .collect(),
            ),
        );
        Ok(output)
    }
}

async fn connect(
    ctx: &TestContext,
    params: &HashMap<String, Value>,
) -> Result<TrustRiskClient, KeywordError> {
    let endpoint = param_or_env(ctx, params, "endpoint", "NVBES_TRUST_RISK_GRPC_ENDPOINT")
        .unwrap_or_else(|| DEFAULT_GRPC_ENDPOINT.to_string());
    let token = param_or_env(
        ctx,
        params,
        "auth_token",
        "NVBES_TRUST_RISK_GRPC_AUTH_TOKEN",
    )
    .ok_or_else(|| {
        KeywordError::InvalidParams(
            "auth_token or NVBES_TRUST_RISK_GRPC_AUTH_TOKEN is required".to_string(),
        )
    })?;
    let environment =
        param_or_env(ctx, params, "environment", "NVBES_ENV").unwrap_or_else(|| "test".to_string());
    crate::environment::validate_test_environment(Some(&environment))
        .map_err(KeywordError::InvalidParams)?;
    crate::environment::validate_loopback_url(&endpoint, "Trust/Risk")
        .map_err(KeywordError::InvalidParams)?;
    let timeout_ms = params
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    let config = TrustRiskClientConfig::from_values(
        &environment,
        endpoint,
        token,
        Duration::from_millis(timeout_ms),
    )
    .map_err(|e| KeywordError::InvalidParams(e.to_string()))?;
    TrustRiskClient::connect(config)
        .await
        .map_err(map_client_error)
}

fn param_or_env(
    ctx: &TestContext,
    params: &HashMap<String, Value>,
    param: &str,
    env_name: &str,
) -> Option<String> {
    params
        .get(param)
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| ctx.variables.env.get(env_name).cloned())
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("trust.submit_signals", Box::new(TrustSubmitSignals));
    registry.register("trust.assess_risk", Box::new(TrustAssessRisk));
    registry.register("trust.submit_labels", Box::new(TrustSubmitLabels));
}

#[path = "test.runner.keywords.trust_risk_kw.signals.rs"]
mod signals;
use signals::*;

#[path = "test.runner.keywords.trust_risk_kw.requests.rs"]
mod requests;
use requests::*;

#[path = "test.runner.keywords.trust_risk_kw.output.rs"]
mod output;
use output::*;

#[cfg(test)]
#[path = "test.runner.keywords.trust_risk_kw.tests.rs"]
mod tests;
