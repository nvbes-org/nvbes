use redis::{AsyncCommands, Script};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{EnqueueJobInput, QueuedJob, keys::*};

const ENQUEUE_SCRIPT: &str = r#"
local use_dedupe = ARGV[3] == "1"
if use_dedupe then
    local existing_id = redis.call("GET", KEYS[3])
    if existing_id then
        if redis.call("EXISTS", ARGV[4] .. existing_id) == 1 then
            return {existing_id, "existing"}
        end
        redis.call("DEL", KEYS[3])
    end
end
if redis.call("EXISTS", KEYS[2]) == 1 then
    if use_dedupe then
        redis.call("SET", KEYS[3], ARGV[1])
    end
    return {ARGV[1], "existing"}
end
redis.call("SET", KEYS[2], ARGV[2])
redis.call("LREM", KEYS[1], 0, ARGV[1])
redis.call("RPUSH", KEYS[1], ARGV[1])
if use_dedupe then
    redis.call("SET", KEYS[3], ARGV[1])
end
return {ARGV[1], "created"}
"#;

const REVIVE_SCRIPT: &str = r#"
if redis.call("GET", KEYS[5]) ~= ARGV[2] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[4])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[4])
end
redis.call("LREM", KEYS[1], 0, ARGV[1])
redis.call("ZREM", KEYS[2], ARGV[1])
redis.call("ZREM", KEYS[3], ARGV[1])
redis.call("ZREM", KEYS[4], ARGV[1])
redis.call("DEL", KEYS[6])
redis.call("SET", KEYS[5], ARGV[3])
if ARGV[4] == "1" and redis.call("GET", KEYS[7]) == ARGV[1] then
    redis.call("PERSIST", KEYS[7])
end
redis.call("RPUSH", KEYS[1], ARGV[1])
return 1
"#;

pub async fn enqueue_job(pool: &RedisPool, input: EnqueueJobInput) -> Result<Uuid, RedisError> {
    let now = now_ts();
    let EnqueueJobInput {
        queue,
        job_type,
        payload,
        idempotency_key,
        max_attempts,
        overwrite_terminal,
        job_id,
    } = input;
    let proposed_id = job_id.unwrap_or_else(Uuid::new_v4);
    let job = QueuedJob {
        id: proposed_id,
        queue: queue.clone(),
        job_type: job_type.clone(),
        idempotency_key: idempotency_key.clone(),
        payload,
        max_attempts: max_attempts.max(1),
        attempts: 0,
        status: STATUS_PENDING.to_string(),
        created_at: now,
        updated_at: now,
        claimed_at: None,
        lease_token: None,
        available_at: now,
        last_error: None,
        result: None,
    };
    let raw = serde_json::to_string(&job)?;
    let dedupe = idempotency_key
        .as_deref()
        .map(|key| dedupe_key(&queue, &job_type, key));
    let unused_dedupe_key = queue_key(&queue, &format!("unused-dedupe:{proposed_id}"));
    let mut conn = pool.get().await?;
    let selected: Vec<String> = Script::new(ENQUEUE_SCRIPT)
        .key(pending_key(&queue))
        .key(job_key(&queue, proposed_id))
        .key(dedupe.as_deref().unwrap_or(&unused_dedupe_key))
        .arg(proposed_id.to_string())
        .arg(raw)
        .arg(if dedupe.is_some() { "1" } else { "0" })
        .arg(queue_key(&queue, "job:"))
        .invoke_async(&mut *conn)
        .await?;
    let selected_id = selected
        .first()
        .ok_or_else(|| RedisError::Connection("enqueue returned no job id".to_string()))
        .and_then(|id| {
            Uuid::parse_str(id).map_err(|error| RedisError::Connection(error.to_string()))
        })?;

    if overwrite_terminal && selected.get(1).is_some_and(|state| state == "existing") {
        revive_terminal_job(pool, &job, selected_id).await?;
    }
    Ok(selected_id)
}

async fn revive_terminal_job(
    pool: &RedisPool,
    replacement: &QueuedJob,
    existing_id: Uuid,
) -> Result<(), RedisError> {
    loop {
        let mut conn = pool.get().await?;
        let current_raw: Option<String> =
            conn.get(job_key(&replacement.queue, existing_id)).await?;
        let Some(current_raw) = current_raw else {
            return Ok(());
        };
        let current: QueuedJob = serde_json::from_str(&current_raw)?;
        if !matches!(current.status.as_str(), STATUS_FAILED | STATUS_DEAD_LETTER) {
            return Ok(());
        }
        let mut revived = replacement.clone();
        revived.id = existing_id;
        let revived_raw = serde_json::to_string(&revived)?;
        let dedupe = replacement
            .idempotency_key
            .as_deref()
            .map(|key| dedupe_key(&replacement.queue, &replacement.job_type, key));
        let unused_dedupe_key =
            queue_key(&replacement.queue, &format!("unused-dedupe:{existing_id}"));
        let changed: i64 = Script::new(REVIVE_SCRIPT)
            .key(pending_key(&replacement.queue))
            .key(running_key(&replacement.queue))
            .key(delayed_key(&replacement.queue))
            .key(dead_letter_key(&replacement.queue))
            .key(job_key(&replacement.queue, existing_id))
            .key(lease_key(&replacement.queue, existing_id))
            .key(dedupe.as_deref().unwrap_or(&unused_dedupe_key))
            .arg(existing_id.to_string())
            .arg(&current_raw)
            .arg(revived_raw)
            .arg(if dedupe.is_some() { "1" } else { "0" })
            .invoke_async(&mut *conn)
            .await?;
        if changed == 1 {
            return Ok(());
        }
    }
}
