use std::{future::Future, net::IpAddr, panic};

use redis::AsyncCommands;
use uuid::Uuid;

use super::keys::KEY_PREFIX;
use crate::RedisPool;

pub async fn redis_pool(max_connections: u32) -> Option<RedisPool> {
    let mut config = crate::config::RedisConfig::from_env();
    if config.url == "redis://localhost:6379" {
        config.url = "redis://127.0.0.1:6379".to_string();
    }
    let environment = std::env::var("NVBES_ENV").ok();
    validate_test_environment(environment.as_deref()).unwrap_or_else(|error| panic!("{error}"));
    validate_loopback_redis(&config.url, environment.as_deref())
        .unwrap_or_else(|error| panic!("{error}"));
    config.max_connections = max_connections;
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        crate::connection::create_pool(&config),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
}

pub async fn with_isolated_queue<F, Fut>(label: &str, max_connections: u32, test: F)
where
    F: FnOnce(RedisPool, String) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Some(pool) = redis_pool(max_connections).await else {
        return;
    };
    let queue = isolated_queue_name(label);
    let outcome = tokio::spawn(test(pool.clone(), queue.clone())).await;
    cleanup_queue(&pool, &queue).await;
    if let Err(join_error) = outcome {
        panic::resume_unwind(join_error.into_panic());
    }
}

pub async fn cleanup_queue(pool: &RedisPool, queue: &str) {
    validate_isolated_queue_name(queue).unwrap_or_else(|error| panic!("{error}"));
    let mut conn = pool.get().await.expect("redis cleanup connection");
    let pattern = format!("{KEY_PREFIX}:{queue}:*");
    let keys = {
        let mut iterator = conn
            .scan_match::<_, String>(&pattern)
            .await
            .expect("scan isolated test keys");
        let mut keys = Vec::new();
        while let Some(key) = iterator.next_item().await {
            keys.push(key);
        }
        keys
    };
    if !keys.is_empty() {
        let _: usize = conn.del(keys).await.expect("delete isolated test keys");
    }
    let mut remaining = conn
        .scan_match::<_, String>(&pattern)
        .await
        .expect("verify isolated test cleanup");
    assert!(
        remaining.next_item().await.is_none(),
        "isolated Redis queue cleanup left keys matching {pattern:?}"
    );
}

fn isolated_queue_name(label: &str) -> String {
    assert!(
        !label.is_empty()
            && label
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()),
        "isolated queue labels must contain only lowercase ASCII letters and digits"
    );
    format!("worker_queue_{label}_{}", Uuid::new_v4())
}

fn validate_isolated_queue_name(queue: &str) -> Result<(), String> {
    let Some(suffix) = queue.strip_prefix("worker_queue_") else {
        return Err(format!(
            "cleanup only accepts generated worker_queue test names; refusing {queue:?}"
        ));
    };
    let Some((label, uuid)) = suffix.rsplit_once('_') else {
        return Err(format!("isolated queue name lacks a UUID: {queue:?}"));
    };
    if label.is_empty()
        || !label
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        || Uuid::parse_str(uuid).is_err()
    {
        return Err(format!("invalid isolated queue name: {queue:?}"));
    }
    Ok(())
}

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
        "worker queue tests require a local/test environment; refusing \
         NVBES_ENV={environment:?}"
    ))
}

fn validate_loopback_redis(redis_url: &str, environment: Option<&str>) -> Result<(), String> {
    let url = redis::parse_redis_url(redis_url)
        .ok_or_else(|| "NVBES_REDIS_URL must be a valid Redis URL".to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "NVBES_REDIS_URL must contain a host".to_string())?;
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    let is_ci_docker_host =
        environment == Some("ci") && host.eq_ignore_ascii_case("host.docker.internal");
    if is_loopback || is_ci_docker_host {
        Ok(())
    } else {
        Err(format!(
            "worker queue integration tests only accept loopback Redis; refusing host {host:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        isolated_queue_name, validate_isolated_queue_name, validate_loopback_redis,
        validate_test_environment,
    };

    #[test]
    fn redis_guard_accepts_the_complete_loopback_address_space() {
        for url in ["redis://localhost:6379", "redis://127.42.0.1:16379/15"] {
            assert!(validate_loopback_redis(url, Some("test")).is_ok(), "{url}");
        }
        assert!(
            validate_loopback_redis("redis://host.docker.internal:6379/15", Some("ci")).is_ok()
        );
    }

    #[test]
    fn redis_guard_rejects_remote_and_deceptive_hosts() {
        for url in [
            "redis://redis.internal:6379",
            "redis://localhost.evil.test:6379",
            "redis://10.0.0.1:6379",
            "https://127.0.0.1:6379",
        ] {
            assert!(validate_loopback_redis(url, Some("test")).is_err(), "{url}");
        }
        assert!(
            validate_loopback_redis("redis://host.docker.internal:6379/15", Some("test")).is_err()
        );
    }

    #[test]
    fn cleanup_guard_accepts_only_generated_unique_queue_names() {
        let first = isolated_queue_name("cleanup");
        let second = isolated_queue_name("cleanup");
        assert_ne!(first, second);
        assert!(validate_isolated_queue_name(&first).is_ok());
        for queue in [
            "email.send",
            "worker_queue_*",
            "worker_queue_cleanup_not-a-uuid",
        ] {
            assert!(validate_isolated_queue_name(queue).is_err(), "{queue}");
        }
    }

    #[test]
    fn environment_guard_rejects_production_like_contexts() {
        assert!(validate_test_environment(None).is_ok());
        assert!(validate_test_environment(Some("test")).is_ok());
        assert!(validate_test_environment(Some("production")).is_err());
        assert!(validate_test_environment(Some("staging")).is_err());
    }
}

#[cfg(test)]
mod cleanup_integration_tests {
    use redis::AsyncCommands;

    use super::{KEY_PREFIX, redis_pool, with_isolated_queue};

    #[tokio::test]
    async fn isolated_queue_is_cleaned_when_the_scenario_panics() {
        let Some(pool_for_spawn) = redis_pool(2).await else {
            return;
        };
        let outcome = tokio::spawn(with_isolated_queue(
            "paniccleanup",
            2,
            |pool, queue| async move {
                let mut connection = pool.get().await.expect("test Redis connection");
                let _: () = connection
                    .set(format!("{KEY_PREFIX}:{queue}:sentinel"), "present")
                    .await
                    .expect("write panic cleanup sentinel");
                panic!("intentional scenario panic");
            },
        ))
        .await;
        drop(pool_for_spawn);
        assert!(outcome.is_err(), "the scenario panic must be propagated");

        let Some(pool) = redis_pool(1).await else {
            return;
        };
        let mut connection = pool.get().await.expect("verification Redis connection");
        let mut remaining = connection
            .scan_match::<_, String>(format!("{KEY_PREFIX}:worker_queue_paniccleanup_*:*"))
            .await
            .expect("scan panic cleanup sentinels");
        assert!(
            remaining.next_item().await.is_none(),
            "panic cleanup left an isolated queue key behind"
        );
    }
}
