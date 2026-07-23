use std::{process::Command, sync::OnceLock, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::time::sleep;

static LOCAL_REDIS_STARTED: OnceLock<()> = OnceLock::new();
static TEST_DATABASE_BOOTSTRAPPED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
static TEST_DATABASE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn uses_local_default_redis() -> bool {
    let url =
        std::env::var("NVBES_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    url == "redis://localhost:6379" || url == "redis://127.0.0.1:6379"
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
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

async fn ensure_local_redis_started() {
    if !uses_local_default_redis() {
        return;
    }

    LOCAL_REDIS_STARTED.get_or_init(|| {
        let mut command = Command::new("redis-server");
        command.args(redis_start_args());
        let _ = command.status();
    });

    let config = nvbes_redis::RedisConfig::from_env();
    wait_for_redis_ready(&config).await;
}

pub async fn ensure_test_redis() {
    ensure_local_redis_started().await;
    let config = nvbes_redis::RedisConfig::from_env();
    wait_for_redis_ready(&config).await;
}

pub async fn test_redis_pool() -> nvbes_redis::RedisPool {
    ensure_test_redis().await;
    let config = nvbes_redis::RedisConfig::from_env();
    nvbes_redis::connection::create_pool(&config)
        .await
        .expect("valid redis pool")
}

pub async fn ensure_test_database(pool: &sqlx::PgPool) {
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

pub fn shared_test_pool() -> PgPool {
    isolated_test_pool(1)
}

pub fn isolated_test_pool(max_connections: u32) -> PgPool {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_lazy(&test_database_url())
        .expect("valid pool")
}

pub fn test_database_lock() -> &'static tokio::sync::Mutex<()> {
    TEST_DATABASE_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}
