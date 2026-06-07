use redis::AsyncCommands;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*};

pub(super) async fn load_job(
    pool: &RedisPool,
    queue: &str,
    job_id: Uuid,
) -> Result<QueuedJob, RedisError> {
    let mut conn = pool.get().await?;
    let raw: String = conn.get(job_key(queue, job_id)).await?;
    Ok(serde_json::from_str(&raw)?)
}

pub async fn get_job(
    pool: &RedisPool,
    queue: &str,
    job_id: Uuid,
) -> Result<Option<QueuedJob>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(job_key(queue, job_id)).await?;
    raw.map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(Into::into)
}

pub(super) async fn save_job(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let raw = serde_json::to_string(job)?;
    let _: () = conn.set(job_key(&job.queue, job.id), raw).await?;
    Ok(())
}

pub(super) async fn move_to_pending(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: usize = conn
        .rpush(pending_key(&job.queue), job.id.to_string())
        .await?;
    Ok(())
}

pub(super) async fn remove_from_indexes(
    pool: &RedisPool,
    job: &QueuedJob,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: usize = conn
        .zrem(running_key(&job.queue), job.id.to_string())
        .await?;
    let _: usize = conn
        .zrem(delayed_key(&job.queue), job.id.to_string())
        .await?;
    Ok(())
}
