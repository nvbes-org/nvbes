use redis::Script;

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*};

const RENEW_LEASE_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) ~= ARGV[1]
    or redis.call("GET", KEYS[2]) ~= ARGV[2]
    or not redis.call("ZSCORE", KEYS[3], ARGV[3]) then
    return 0
end
redis.call("ZADD", KEYS[3], ARGV[4], ARGV[3])
return 1
"#;

pub async fn renew_job_lease(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let lease_token = job.lease_token.ok_or_else(|| RedisError::JobLeaseLost {
        queue: job.queue.clone(),
        job_id: job.id,
    })?;
    let raw = serde_json::to_string(job)?;
    let mut conn = pool.get().await?;
    let renewed: u64 = Script::new(RENEW_LEASE_SCRIPT)
        .key(job_key(&job.queue, job.id))
        .key(lease_key(&job.queue, job.id))
        .key(running_key(&job.queue))
        .arg(raw)
        .arg(lease_token.to_string())
        .arg(job.id.to_string())
        .arg(now_ts())
        .invoke_async(&mut *conn)
        .await?;
    if renewed == 1 {
        Ok(())
    } else {
        Err(RedisError::JobLeaseLost {
            queue: job.queue.clone(),
            job_id: job.id,
        })
    }
}
