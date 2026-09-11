use std::collections::HashMap;
use std::time::Duration;

use crate::runner::{
    TestContext,
    registry::{Keyword, KeywordError, KeywordRegistry},
    resources::psql,
};
use async_trait::async_trait;
use serde_json::{Value, json};

struct SetupDb;
struct Migrate;
struct StartService;
struct HealthCheck;
struct Cleanup;

fn required<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, KeywordError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| KeywordError::InvalidParams(format!("missing string parameter: {key}")))
}

fn success() -> HashMap<String, Value> {
    HashMap::from([("status".into(), json!(200))])
}

#[async_trait]
impl Keyword for SetupDb {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = required(params, "database_url")?;
        crate::environment::validate_loopback_url(url, "PostgreSQL")
            .map_err(KeywordError::InvalidParams)?;
        let name = format!("nvbes_test_{}", uuid::Uuid::new_v4().simple());
        ctx.resources
            .lock()
            .await
            .databases
            .push((url.into(), name.clone()));
        psql(url, &format!("CREATE DATABASE \"{name}\"")).await?;
        let mut database_url = reqwest::Url::parse(url)
            .map_err(|_| KeywordError::InvalidParams("invalid PostgreSQL URL".into()))?;
        database_url.set_path(&name);
        let mut output = success();
        output.insert("database_name".into(), json!(name));
        output.insert("database_url".into(), json!(database_url.as_str()));
        Ok(output)
    }
}

#[async_trait]
impl Keyword for Migrate {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = required(params, "database_url")?;
        let parsed = reqwest::Url::parse(url)
            .map_err(|_| KeywordError::InvalidParams("invalid PostgreSQL URL".into()))?;
        crate::environment::validate_loopback_url(url, "PostgreSQL")
            .map_err(KeywordError::InvalidParams)?;
        let name = parsed.path().trim_start_matches('/');
        if !ctx
            .resources
            .lock()
            .await
            .databases
            .iter()
            .any(|(origin, owned)| {
                reqwest::Url::parse(origin).is_ok_and(|mut expected| {
                    expected.set_path(owned);
                    owned == name && expected == parsed
                })
            })
        {
            return Err(KeywordError::InvalidParams(
                "migrations require a database owned by this scenario".into(),
            ));
        }
        let directory = required(params, "migrations_dir")?;
        let output = tokio::process::Command::new("sqlx")
            .args(["migrate", "run", "--source", directory])
            .env("DATABASE_URL", url)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|_| KeywordError::Execution("migration command could not execute".into()))?;
        if !output.status.success() {
            return Err(KeywordError::Execution("test migration failed".into()));
        }
        Ok(success())
    }
}

#[async_trait]
impl Keyword for StartService {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        if params.contains_key("start_command") {
            return Err(KeywordError::InvalidParams(
                "shell start_command is not supported".into(),
            ));
        }
        let name = required(params, "service_name")?;
        if ![
            "identity-service",
            "account-service",
            "billing-service",
            "email-worker",
            "trust-risk-service",
            "platform",
        ]
        .contains(&name)
        {
            return Err(KeywordError::InvalidParams("unknown V1 service".into()));
        }
        let url = required(params, "health_url")?;
        crate::environment::validate_loopback_url(url, "health")
            .map_err(KeywordError::InvalidParams)?;
        let executable = if name == "platform" {
            "target/debug/nvbes-platform-operations".to_string()
        } else if name == "billing-service" {
            "apps/billing-service/target/debug/nvbes-billing-service".to_string()
        } else {
            format!("target/debug/nvbes-{name}")
        };
        let arguments = match params.get("args") {
            None => vec![],
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| {
                    value.as_str().ok_or_else(|| {
                        KeywordError::InvalidParams("service args must be strings".into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err(KeywordError::InvalidParams(
                    "service args must be an array".into(),
                ));
            }
        };
        let child = tokio::process::Command::new(executable)
            .args(arguments)
            .env_clear()
            .envs(&ctx.variables.env)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| KeywordError::Execution("compiled test service could not start".into()))?;
        ctx.resources.lock().await.children.push(child);
        let client = local_client()?;
        loop {
            if client
                .get(url)
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
            {
                return Ok(success());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[async_trait]
impl Keyword for HealthCheck {
    async fn execute(
        &self,
        _: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let url = required(params, "url")?;
        crate::environment::validate_loopback_url(url, "health")
            .map_err(KeywordError::InvalidParams)?;
        let response = local_client()?
            .get(url)
            .send()
            .await
            .map_err(|_| KeywordError::Execution("health request failed".into()))?;
        Ok(HashMap::from([
            ("status".into(), json!(response.status().as_u16())),
            ("healthy".into(), json!(response.status().is_success())),
        ]))
    }
}

#[async_trait]
impl Keyword for Cleanup {
    async fn execute(
        &self,
        ctx: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        if !params.is_empty() {
            return Err(KeywordError::InvalidParams(
                "cleanup accepts only resources owned by the scenario, no parameters".into(),
            ));
        }
        ctx.resources.lock().await.cleanup().await?;
        Ok(success())
    }
}

fn local_client() -> Result<reqwest::Client, KeywordError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|_| KeywordError::Execution("HTTP client initialization failed".into()))
}

pub fn register(registry: &mut KeywordRegistry) {
    registry.register("infra.setup_db", Box::new(SetupDb));
    registry.register("infra.migrate", Box::new(Migrate));
    registry.register("infra.start_service", Box::new(StartService));
    registry.register("infra.health_check", Box::new(HealthCheck));
    registry.register("infra.cleanup", Box::new(Cleanup));
}
