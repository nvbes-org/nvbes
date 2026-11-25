use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::runner::TestContext;
use crate::runner::registry::{Keyword, KeywordError, KeywordRegistry};

struct HealthProbe;

#[async_trait]
impl Keyword for HealthProbe {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let base_url = require_string(params, "base_url")?.trim_end_matches('/');
        let endpoint = params
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("ready");
        let timeout_ms = optional_u64(params, "timeout_ms").unwrap_or(3000);
        let url = format!("{base_url}/health/{endpoint}");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| KeywordError::Execution(format!("http client: {e}")))?;
        let start = std::time::Instant::now();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("health probe failed: {e}")))?;
        let latency_ms = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let live = endpoint == "live" && status == 200;
        let ready = endpoint == "ready" && status == 200;
        let healthy = status == 200;
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        output.insert("live".to_string(), Value::Bool(live));
        output.insert("ready".to_string(), Value::Bool(ready));
        output.insert("healthy".to_string(), Value::Bool(healthy));
        output.insert("latency_ms".to_string(), Value::Number(latency_ms.into()));
        output.insert("url".to_string(), Value::String(url));
        Ok(output)
    }
}

struct CockpitOverview;

#[async_trait]
impl Keyword for CockpitOverview {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let base_url = require_string(params, "base_url")?.trim_end_matches('/');
        let auth_token = require_string(params, "auth_token")?;
        let timeout_ms = optional_u64(params, "timeout_ms").unwrap_or(5000);
        let url = format!("{base_url}/api/v1/overview");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| KeywordError::Execution(format!("http client: {e}")))?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {auth_token}"))
            .send()
            .await
            .map_err(|e| KeywordError::Execution(format!("cockpit overview failed: {e}")))?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let mut output = HashMap::new();
        output.insert("status".to_string(), Value::Number(status.into()));
        output.insert("body".to_string(), body);
        Ok(output)
    }
}

struct BudgetStage;

#[async_trait]
impl Keyword for BudgetStage {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let spend_cents = params
            .get("spend_cents")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                KeywordError::InvalidParams("missing required param: spend_cents".to_string())
            })? as u32;
        let disable_threshold =
            optional_u64(params, "disable_non_essential_cents").unwrap_or(2500) as u32;
        let freeze_threshold =
            optional_u64(params, "freeze_cost_creation_cents").unwrap_or(2800) as u32;
        let essential_threshold =
            optional_u64(params, "essential_only_cents").unwrap_or(3000) as u32;
        let stage = if spend_cents >= essential_threshold {
            "essential_only"
        } else if spend_cents >= freeze_threshold {
            "freeze_cost_creation"
        } else if spend_cents >= disable_threshold {
            "disable_non_essential"
        } else {
            "normal"
        };
        let within_budget = spend_cents < essential_threshold;
        let mut output = HashMap::new();
        output.insert("stage".to_string(), Value::String(stage.to_string()));
        output.insert("within_budget".to_string(), Value::Bool(within_budget));
        output.insert("spend_cents".to_string(), Value::Number(spend_cents.into()));
        output.insert(
            "essential_only_cents".to_string(),
            Value::Number(essential_threshold.into()),
        );
        Ok(output)
    }
}

struct DegradedProcedure;

#[async_trait]
impl Keyword for DegradedProcedure {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let scenario = require_string(params, "scenario")?;
        let procedure = match scenario {
            "identity_unavailable" => degraded_procedure_json(
                "Identity Service Unavailable",
                "SEV1",
                "JWT token verification fails or Identity database is unreachable.",
                "Maintain read-only for verified cached tokens; reject sensitive mutations with 503.",
                vec![
                    "Verify Scaleway serverless container status for identity-service.",
                    "Check PostgreSQL connection pool for Identity database.",
                    "Do NOT bypass JWT validation in downstream services.",
                    "Engage Identity recovery runbook if container is crashlooping.",
                ],
                vec![
                    "Verify /health/live and /health/ready on identity-service return 200.",
                    "Test synthetic auth token generation.",
                ],
            ),
            "account_unavailable" => degraded_procedure_json(
                "Account Service Unavailable",
                "SEV2",
                "Profile updates, team memberships or privacy exports failing.",
                "Queue pending mutations; return read-only profile state where cached.",
                vec![
                    "Inspect account-service error logs and metrics.",
                    "Ensure ongoing GDPR export jobs are safely suspended without data loss.",
                ],
                vec![
                    "Verify account-service health probes.",
                    "Check team membership and profile read queries succeed.",
                ],
            ),
            "stripe_unavailable" => degraded_procedure_json(
                "Stripe API / Webhooks Unavailable",
                "SEV2",
                "Test Stripe API unreachable or webhook delivery failing.",
                "Mark all pending checkouts as pending; NEVER cut off access automatically.",
                vec![
                    "Check Stripe status page and webhook signing secret.",
                    "Verify webhook outbox in billing-service.",
                    "Do NOT manually modify customer entitlements without logged audit.",
                ],
                vec!["Send signed test webhook event and verify processed status."],
            ),
            "email_delayed" => degraded_procedure_json(
                "Email Delivery Delayed or Queued",
                "SEV2",
                "Scaleway TEM or worker experiencing backlog or delivery retries.",
                "Messages remain durably queued with exponential backoff; dead-letter after max attempts.",
                vec![
                    "Inspect email-worker queue depth and provider status.",
                    "Verify suppressions list for false positives.",
                ],
                vec!["Trigger synthetic transactional email and monitor dispatch time."],
            ),
            "trust_risk_unavailable" => degraded_procedure_json(
                "Trust/Risk Service Unavailable",
                "SEV3",
                "Trust/Risk gRPC evaluation endpoint unreachable.",
                "Shadow mode default: allow operation, log signal for deferred batch evaluation.",
                vec![
                    "Verify trust-risk-service health probe.",
                    "Confirm downstream services continue with shadow allow.",
                ],
                vec!["Verify trust_risk gRPC ping and review cases queue."],
            ),
            "postgres_in_restore" => degraded_procedure_json(
                "PostgreSQL Database in Restoration",
                "SEV1",
                "Restoration exercise or incident recovery in progress on database.",
                "Application transitions to maintenance mode; freeze all write traffic.",
                vec![
                    "Confirm target is isolated non-production target or verified restore.",
                    "Ensure RPO (<= 24h) and RTO (<= 8h) are tracked.",
                    "Execute schema and integrity verification script.",
                ],
                vec!["Validate checksums, migration version, and record integrity."],
            ),
            "finops_threshold_exceeded" => degraded_procedure_json(
                "FinOps Spend Threshold Exceeded",
                "SEV2",
                "Monthly spend projected or confirmed to exceed budget stages.",
                "Apply automatic stage guardrails (25 EUR disable non-essential, 28 EUR freeze cost creation, 30 EUR essential only).",
                vec![
                    "Check provider invoices and serverless container instance counts.",
                    "Confirm min_scale is 0 on all services.",
                    "Verify non-essential background jobs are stopped.",
                ],
                vec!["Run pnpm check:finops and verify spend stays strictly <= 30 EUR TTC."],
            ),
            other => {
                return Err(KeywordError::InvalidParams(format!(
                    "unknown degraded scenario: {other}"
                )));
            }
        };
        Ok(procedure)
    }
}

struct ServiceReadiness;

#[async_trait]
impl Keyword for ServiceReadiness {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let services = params
            .get("services")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                KeywordError::InvalidParams(
                    "missing required param: services (object mapping name to base_url)"
                        .to_string(),
                )
            })?;
        let timeout_ms = optional_u64(params, "timeout_ms").unwrap_or(3000);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| KeywordError::Execution(format!("http client: {e}")))?;
        let mut service_results = serde_json::Map::new();
        let mut all_ready = true;
        for (name, url_val) in services {
            let base_url = url_val.as_str().unwrap_or("").trim_end_matches('/');
            crate::environment::validate_loopback_url(base_url, "readiness")
                .map_err(KeywordError::InvalidParams)?;
            let url = format!("{base_url}/health/ready");
            let start = std::time::Instant::now();
            let status = match client.get(&url).send().await {
                Ok(resp) => resp.status().as_u16(),
                Err(_) => {
                    all_ready = false;
                    0
                }
            };
            let latency_ms = start.elapsed().as_millis() as u64;
            let ready = status == 200;
            if !ready {
                all_ready = false;
            }
            let mut entry = serde_json::Map::new();
            entry.insert("status".to_string(), Value::Number(status.into()));
            entry.insert("ready".to_string(), Value::Bool(ready));
            entry.insert("latency_ms".to_string(), Value::Number(latency_ms.into()));
            service_results.insert(name.clone(), Value::Object(entry));
        }
        let mut output = HashMap::new();
        output.insert("overall_ready".to_string(), Value::Bool(all_ready));
        output.insert("services".to_string(), Value::Object(service_results));
        Ok(output)
    }
}

struct WaitForReady;

#[async_trait]
impl Keyword for WaitForReady {
    async fn execute(
        &self,
        _ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let base_url = require_string(params, "base_url")?.trim_end_matches('/');
        let timeout_ms = optional_u64(params, "timeout_ms").unwrap_or(30000);
        let interval_ms = optional_u64(params, "interval_ms").unwrap_or(1000);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(std::time::Duration::from_millis(interval_ms.min(3000)))
            .build()
            .map_err(|e| KeywordError::Execution(format!("http client: {e}")))?;
        let url = format!("{base_url}/health/ready");
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut attempts = 0u32;
        let start = std::time::Instant::now();
        loop {
            attempts += 1;
            let status = match client.get(&url).send().await {
                Ok(resp) => resp.status().as_u16(),
                Err(_) => 0,
            };
            if status == 200 {
                let latency_ms = start.elapsed().as_millis() as u64;
                let mut output = HashMap::new();
                output.insert("ready".to_string(), Value::Bool(true));
                output.insert("attempts".to_string(), Value::Number(attempts.into()));
                output.insert("latency_ms".to_string(), Value::Number(latency_ms.into()));
                output.insert("status".to_string(), Value::Number(status.into()));
                return Ok(output);
            }
            if std::time::Instant::now() >= deadline {
                let latency_ms = start.elapsed().as_millis() as u64;
                let mut output = HashMap::new();
                output.insert("ready".to_string(), Value::Bool(false));
                output.insert("attempts".to_string(), Value::Number(attempts.into()));
                output.insert("latency_ms".to_string(), Value::Number(latency_ms.into()));
                output.insert("status".to_string(), Value::Number(status.into()));
                return Ok(output);
            }
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }
    }
}

fn degraded_procedure_json(
    title: &str,
    severity: &str,
    description: &str,
    automatic_fallback: &str,
    operator_checklist: Vec<&str>,
    verification_steps: Vec<&str>,
) -> HashMap<String, Value> {
    let mut output = HashMap::new();
    output.insert("title".to_string(), Value::String(title.to_string()));
    output.insert("severity".to_string(), Value::String(severity.to_string()));
    output.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    output.insert(
        "automatic_fallback".to_string(),
        Value::String(automatic_fallback.to_string()),
    );
    output.insert(
        "operator_checklist".to_string(),
        Value::Array(
            operator_checklist
                .into_iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
    );
    output.insert(
        "verification_steps".to_string(),
        Value::Array(
            verification_steps
                .into_iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
    );
    output
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

fn optional_u64(params: &HashMap<String, Value>, key: &str) -> Option<u64> {
    params.get(key).and_then(|v| v.as_u64())
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("platform.health_probe", Box::new(HealthProbe));
    registry.register("platform.cockpit_overview", Box::new(CockpitOverview));
    registry.register("platform.budget_stage", Box::new(BudgetStage));
    registry.register("platform.degraded_procedure", Box::new(DegradedProcedure));
    registry.register("platform.service_readiness", Box::new(ServiceReadiness));
    registry.register("platform.wait_for_ready", Box::new(WaitForReady));
}

#[cfg(test)]
#[path = "test.runner.keywords.platform_kw.tests.rs"]
mod tests;
