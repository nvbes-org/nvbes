use std::{net::IpAddr, process::Command, sync::OnceLock, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::time::sleep;
use url::Url;

#[cfg(test)]
#[path = "identity.test_support.cloud_mock.rs"]
pub mod cloud_mock;

static LOCAL_REDIS_STARTED: OnceLock<()> = OnceLock::new();
static TEST_DATABASE_BOOTSTRAPPED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
static TEST_DATABASE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
const DESTRUCTIVE_TEST_OPT_IN: &str = "account-quality-v1";

fn validate_test_environment(environment: Option<&str>) -> Result<(), String> {
    let Some(environment) = environment.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if matches!(
        environment.to_ascii_lowercase().as_str(),
        "ci" | "dev" | "development" | "integration" | "local" | "test" | "testing"
    ) {
        return Ok(());
    }
    Err(format!(
        "test resources require a local/test environment; refusing NVBES_ENV={environment:?}"
    ))
}

fn validate_loopback_host(host: &str, resource: &str) -> Result<(), String> {
    let normalized_host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let is_loopback = normalized_host.eq_ignore_ascii_case("localhost")
        || normalized_host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if is_loopback {
        Ok(())
    } else {
        Err(format!(
            "{resource} tests only accept loopback hosts; refusing {host:?}"
        ))
    }
}

#[doc(hidden)]
pub fn validate_test_database_url(
    database_url: &str,
    environment: Option<&str>,
    destructive_opt_in: Option<&str>,
) -> Result<(), String> {
    validate_test_environment(environment)?;
    if destructive_opt_in != Some(DESTRUCTIVE_TEST_OPT_IN) {
        return Err(format!(
            "destructive Account tests require \
             NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE={DESTRUCTIVE_TEST_OPT_IN}"
        ));
    }
    let url =
        Url::parse(database_url).map_err(|error| format!("invalid PostgreSQL URL: {error}"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(format!(
            "test database URL must use postgres or postgresql, got {:?}",
            url.scheme()
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| "test database URL must contain a host".to_string())?;
    validate_loopback_host(host, "PostgreSQL")?;
    validate_test_database_name(url.path().trim_start_matches('/'))
}

fn validate_test_database_name(database_name: &str) -> Result<(), String> {
    let normalized = database_name.to_ascii_lowercase();
    let segments = || {
        normalized
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|segment| !segment.is_empty())
    };
    if database_name.is_empty()
        || matches!(normalized.as_str(), "postgres" | "template0" | "template1")
        || !segments().any(|segment| segment == "test")
        || segments().any(|segment| matches!(segment, "live" | "prod" | "production" | "staging"))
    {
        return Err(format!(
            "test database must be disposable, contain a `test` segment, and not name a \
             production-like environment; refusing \
             {database_name:?}"
        ));
    }
    Ok(())
}

fn validate_test_redis_url(redis_url: &str, environment: Option<&str>) -> Result<(), String> {
    validate_test_environment(environment)?;
    let url = Url::parse(redis_url).map_err(|error| format!("invalid Redis URL: {error}"))?;
    if !matches!(url.scheme(), "redis" | "rediss") {
        return Err(format!(
            "test Redis URL must use redis or rediss, got {:?}",
            url.scheme()
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| "test Redis URL must contain a host".to_string())?;
    validate_loopback_host(host, "Redis")
}

fn test_environment() -> Option<String> {
    std::env::var("NVBES_ENV").ok()
}

fn destructive_test_opt_in() -> Option<String> {
    std::env::var("NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE").ok()
}

fn validated_test_redis_config() -> nvbes_redis::RedisConfig {
    let config = nvbes_redis::RedisConfig::from_env();
    validate_test_redis_url(&config.url, test_environment().as_deref())
        .unwrap_or_else(|error| panic!("{error}"));
    config
}

fn uses_local_default_redis(redis_url: &str) -> bool {
    matches!(
        redis_url,
        "redis://localhost:6379" | "redis://127.0.0.1:6379"
    )
}

fn redis_start_args() -> Vec<String> {
    let mut args = vec![
        "--save".to_string(),
        "".to_string(),
        "--appendonly".to_string(),
        "no".to_string(),
        "--daemonize".to_string(),
        "yes".to_string(),
        "--bind".to_string(),
        "127.0.0.1".to_string(),
    ];

    if let Ok(password) = std::env::var("NVBES_REDIS_PASSWORD")
        && !password.trim().is_empty()
    {
        args.push("--requirepass".to_string());
        args.push(password);
    }

    args
}

async fn wait_for_redis_ready(config: &nvbes_redis::RedisConfig) {
    for _ in 0..20 {
        if let Ok(pool) = nvbes_redis::connection::create_pool(config).await
            && nvbes_redis::connection::health_check(&pool).await.is_ok()
        {
            return;
        }

        sleep(Duration::from_millis(200)).await;
    }

    panic!("Redis is not reachable at {}", config.url);
}

fn test_database_url() -> String {
    let database_url = std::env::var("NVBES_IDENTITY_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes_test".to_string());
    validate_test_database_url(
        &database_url,
        test_environment().as_deref(),
        destructive_test_opt_in().as_deref(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    database_url
}

async fn ensure_local_redis_started(config: &nvbes_redis::RedisConfig) {
    if !uses_local_default_redis(&config.url) {
        return;
    }

    LOCAL_REDIS_STARTED.get_or_init(|| {
        let mut command = Command::new("redis-server");
        command.args(redis_start_args());
        let _ = command.status();
    });

    wait_for_redis_ready(config).await;
}

pub async fn ensure_test_redis() {
    let config = validated_test_redis_config();
    ensure_local_redis_started(&config).await;
    wait_for_redis_ready(&config).await;
}

pub async fn test_redis_pool() -> nvbes_redis::RedisPool {
    let config = validated_test_redis_config();
    ensure_local_redis_started(&config).await;
    wait_for_redis_ready(&config).await;
    nvbes_redis::connection::create_pool(&config)
        .await
        .expect("valid redis pool")
}

pub async fn ensure_test_database(pool: &sqlx::PgPool) {
    validate_test_database_connection(pool).await;
    TEST_DATABASE_BOOTSTRAPPED
        .get_or_init(move || async move {
            sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
                .execute(pool)
                .await
                .expect("public schema should exist");
            sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
                .execute(pool)
                .await
                .expect("pgcrypto extension should exist");
            crate::database::run_migrations(pool)
                .await
                .expect("migrations should run");
        })
        .await;
}

async fn validate_test_database_connection(pool: &PgPool) {
    validate_test_environment(test_environment().as_deref())
        .unwrap_or_else(|error| panic!("{error}"));
    if destructive_test_opt_in().as_deref() != Some(DESTRUCTIVE_TEST_OPT_IN) {
        panic!(
            "destructive Account tests require \
             NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE={DESTRUCTIVE_TEST_OPT_IN}"
        );
    }
    let options = pool.connect_options();
    validate_loopback_host(options.get_host(), "PostgreSQL")
        .unwrap_or_else(|error| panic!("{error}"));
    let configured_database = options
        .get_database()
        .expect("test database connection must name a database");
    validate_test_database_name(configured_database).unwrap_or_else(|error| panic!("{error}"));

    let connected_database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(pool)
        .await
        .expect("test database identity must be readable before migrations");
    assert_eq!(
        connected_database, configured_database,
        "connected test database must match the validated pool configuration"
    );
}

pub fn shared_test_pool() -> PgPool {
    isolated_test_pool(10)
}

pub fn isolated_test_pool(max_connections: u32) -> PgPool {
    PgPoolOptions::new()
        .max_connections(max_connections.max(10))
        .acquire_timeout(Duration::from_secs(10))
        .connect_lazy(&test_database_url())
        .expect("valid pool")
}

pub fn test_database_lock() -> &'static tokio::sync::Mutex<()> {
    TEST_DATABASE_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[cfg(test)]
mod resource_guard_tests {
    use super::{validate_test_database_url, validate_test_environment, validate_test_redis_url};

    #[test]
    fn database_guard_accepts_only_loopback_disposable_test_databases() {
        for url in [
            "postgres://user:secret@localhost:5432/nvbes_test",
            "postgresql://user:secret@127.42.0.1:5432/account_integration_test",
            "postgres://user:secret@[::1]:5432/test_account",
        ] {
            assert!(
                validate_test_database_url(url, Some("test"), Some("account-quality-v1")).is_ok()
            );
        }
    }

    #[test]
    fn database_guard_rejects_remote_shared_and_production_targets() {
        for url in [
            "postgres://user:secret@db.example.test:5432/nvbes_test",
            "postgres://user:secret@localhost.evil.test:5432/nvbes_test",
            "postgres://user:secret@127.0.0.1:5432/nvbes",
            "postgres://user:secret@127.0.0.1:5432/postgres",
            "postgres://user:secret@127.0.0.1:5432/nvbes_production_test",
        ] {
            assert!(
                validate_test_database_url(url, Some("test"), Some("account-quality-v1")).is_err()
            );
        }
        assert!(
            validate_test_database_url(
                "postgres://user:secret@127.0.0.1:5432/nvbes_test",
                Some("production"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
        assert!(
            validate_test_database_url(
                "postgres://user:secret@127.0.0.1:5432/latest_production",
                Some("test"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
        assert!(
            validate_test_database_url(
                "postgres://user:secret@127.0.0.1:5432/nvbes_test",
                Some("test"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn redis_guard_rejects_non_loopback_and_non_test_environments() {
        assert!(validate_test_redis_url("redis://127.0.0.1:6379/15", Some("ci")).is_ok());
        assert!(validate_test_redis_url("rediss://[::1]:6380/15", Some("test")).is_ok());
        assert!(validate_test_redis_url("redis://redis.internal:6379", Some("test")).is_err());
        assert!(validate_test_redis_url("redis://localhost.evil.test:6379", Some("test")).is_err());
        assert!(validate_test_redis_url("redis://127.0.0.1:6379", Some("staging")).is_err());
    }

    #[test]
    fn environment_guard_fails_closed_for_unknown_values() {
        assert!(validate_test_environment(None).is_ok());
        assert!(validate_test_environment(Some("development")).is_ok());
        assert!(validate_test_environment(Some("preview")).is_err());
    }
}
