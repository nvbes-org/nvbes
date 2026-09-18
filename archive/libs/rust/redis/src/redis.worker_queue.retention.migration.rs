use std::time::Duration;

use redis::{AsyncCommands, Script};
use tokio::time::Instant;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*, retention::*};

const SCAN_BATCH_SIZE: usize = 250;
const MIGRATION_LOCK_TTL_SECONDS: u64 = 30;
const MIGRATION_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const MIGRATION_WAIT_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetentionMigrationOutcome {
    Completed,
    AlreadyComplete,
    WaitedForCompletion,
}

const MIGRATE_TERMINAL_JOB_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) ~= ARGV[1] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[3])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[3])
end
local job_ttl = redis.call("TTL", KEYS[1])
if job_ttl < 1 or job_ttl > tonumber(ARGV[4]) then
    job_ttl = tonumber(ARGV[4])
end
redis.call("SET", KEYS[1], ARGV[2])
redis.call("EXPIRE", KEYS[1], job_ttl)
redis.call("LREM", KEYS[5], 0, ARGV[3])
redis.call("ZREM", KEYS[6], ARGV[3])
redis.call("ZREM", KEYS[7], ARGV[3])
redis.call("DEL", KEYS[4])
if ARGV[6] == "1" and redis.call("GET", KEYS[2]) == ARGV[3] then
    local dedupe_ttl = redis.call("TTL", KEYS[2])
    if dedupe_ttl < 1 or dedupe_ttl > tonumber(ARGV[4]) then
        dedupe_ttl = tonumber(ARGV[4])
    end
    redis.call("EXPIRE", KEYS[2], dedupe_ttl)
end
redis.call("ZREMRANGEBYSCORE", KEYS[3], "-inf", ARGV[7])
redis.call("ZREM", KEYS[3], ARGV[3])
if ARGV[5] == "dead_letter" then
    redis.call("ZADD", KEYS[3], ARGV[7] + job_ttl, ARGV[3])
    redis.call("EXPIRE", KEYS[3], ARGV[4])
end
return 1
"#;

const RENEW_LOCK_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) ~= ARGV[1] then
    return 0
end
redis.call("EXPIRE", KEYS[1], ARGV[2])
return 1
"#;

const COMPLETE_MIGRATION_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) ~= ARGV[1] then
    return 0
end
redis.call("SET", KEYS[2], "complete")
redis.call("DEL", KEYS[1])
return 1
"#;

const RELEASE_LOCK_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) ~= ARGV[1] then
    return 0
end
return redis.call("DEL", KEYS[1])
"#;

pub(super) async fn enforce_legacy_terminal_retention(
    pool: &RedisPool,
    queue: &str,
) -> Result<RetentionMigrationOutcome, RedisError> {
    let marker = migration_marker_key(queue);
    let lock = migration_lock_key(queue);
    let mut conn = pool.get().await?;
    if conn.exists(&marker).await? {
        return Ok(RetentionMigrationOutcome::AlreadyComplete);
    }

    let deadline = Instant::now() + MIGRATION_WAIT_TIMEOUT;
    let owner = Uuid::new_v4().to_string();
    loop {
        let acquired: Option<String> = redis::cmd("SET")
            .arg(&lock)
            .arg(&owner)
            .arg("NX")
            .arg("EX")
            .arg(MIGRATION_LOCK_TTL_SECONDS)
            .query_async(&mut *conn)
            .await?;
        if acquired.is_some() {
            match conn.exists(&marker).await {
                Ok(true) => {
                    release_lock(&mut conn, &lock, &owner).await;
                    return Ok(RetentionMigrationOutcome::WaitedForCompletion);
                }
                Ok(false) => {}
                Err(error) => {
                    release_lock(&mut conn, &lock, &owner).await;
                    return Err(error.into());
                }
            }
            break;
        }
        if conn.exists(&marker).await? {
            return Ok(RetentionMigrationOutcome::WaitedForCompletion);
        }
        if Instant::now() >= deadline {
            return Err(RedisError::OperationTimeout(
                "worker_queue_terminal_retention_migration_lock",
            ));
        }
        tokio::time::sleep(MIGRATION_WAIT_INTERVAL).await;
    }

    let migration = migrate_legacy_terminal_jobs(&mut conn, queue, &lock, &owner).await;
    if let Err(error) = migration {
        release_lock(&mut conn, &lock, &owner).await;
        return Err(error);
    }

    let completed: u64 = Script::new(COMPLETE_MIGRATION_SCRIPT)
        .key(&lock)
        .key(&marker)
        .arg(&owner)
        .invoke_async(&mut *conn)
        .await?;
    if completed == 1 {
        Ok(RetentionMigrationOutcome::Completed)
    } else {
        Err(RedisError::OperationTimeout(
            "worker_queue_terminal_retention_migration_lock",
        ))
    }
}

async fn migrate_legacy_terminal_jobs(
    conn: &mut bb8_redis::bb8::PooledConnection<'_, bb8_redis::RedisConnectionManager>,
    queue: &str,
    lock: &str,
    owner: &str,
) -> Result<(), RedisError> {
    let retention_seconds = terminal_retention_seconds();
    let pattern = queue_key(queue, "job:*");
    let mut cursor = 0_u64;
    loop {
        renew_lock(conn, lock, owner).await?;
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(SCAN_BATCH_SIZE)
            .query_async(&mut **conn)
            .await?;
        if !keys.is_empty() {
            let values: Vec<Option<String>> = redis::cmd("MGET")
                .arg(&keys)
                .query_async(&mut **conn)
                .await?;
            for (index, (key, raw)) in keys
                .into_iter()
                .zip(values)
                .filter_map(|(key, raw)| raw.map(|raw| (key, raw)))
                .enumerate()
            {
                if index > 0 && index % 50 == 0 {
                    renew_lock(conn, lock, owner).await?;
                }
                migrate_terminal_job(conn, queue, &key, &raw, retention_seconds).await?;
            }
        }
        if next_cursor == 0 {
            break;
        }
        cursor = next_cursor;
    }
    Ok(())
}

async fn renew_lock(
    conn: &mut bb8_redis::bb8::PooledConnection<'_, bb8_redis::RedisConnectionManager>,
    lock: &str,
    owner: &str,
) -> Result<(), RedisError> {
    let renewed: u64 = Script::new(RENEW_LOCK_SCRIPT)
        .key(lock)
        .arg(owner)
        .arg(MIGRATION_LOCK_TTL_SECONDS)
        .invoke_async(&mut **conn)
        .await?;
    if renewed == 1 {
        Ok(())
    } else {
        Err(RedisError::OperationTimeout(
            "worker_queue_terminal_retention_migration_lock",
        ))
    }
}

async fn release_lock(
    conn: &mut bb8_redis::bb8::PooledConnection<'_, bb8_redis::RedisConnectionManager>,
    lock: &str,
    owner: &str,
) {
    let _: Result<u64, _> = Script::new(RELEASE_LOCK_SCRIPT)
        .key(lock)
        .arg(owner)
        .invoke_async(&mut **conn)
        .await;
}

pub(super) fn migration_marker_key(queue: &str) -> String {
    queue_key(queue, "terminal-retention-v2-complete")
}

pub(super) fn migration_lock_key(queue: &str) -> String {
    queue_key(queue, "terminal-retention-v2-lock")
}

async fn migrate_terminal_job(
    conn: &mut bb8_redis::bb8::PooledConnection<'_, bb8_redis::RedisConnectionManager>,
    queue: &str,
    key: &str,
    raw: &str,
    retention_seconds: u64,
) -> Result<(), RedisError> {
    let Ok(job) = serde_json::from_str::<QueuedJob>(raw) else {
        return Ok(());
    };
    let Some(snapshot) = existing_terminal_snapshot(&job) else {
        return Ok(());
    };
    let dedupe = job
        .idempotency_key
        .as_deref()
        .map(|value| dedupe_key(queue, &job.job_type, value));
    let unused_dedupe_key = queue_key(queue, &format!("unused-dedupe:{}", job.id));
    let sanitized = serde_json::to_string(&snapshot)?;
    let _: u64 = Script::new(MIGRATE_TERMINAL_JOB_SCRIPT)
        .key(key)
        .key(dedupe.as_deref().unwrap_or(&unused_dedupe_key))
        .key(dead_letter_key(queue))
        .key(lease_key(queue, job.id))
        .key(pending_key(queue))
        .key(running_key(queue))
        .key(delayed_key(queue))
        .arg(raw)
        .arg(sanitized)
        .arg(job.id.to_string())
        .arg(retention_seconds)
        .arg(&job.status)
        .arg(if dedupe.is_some() { "1" } else { "0" })
        .arg(now_ts())
        .invoke_async(&mut **conn)
        .await?;
    Ok(())
}
