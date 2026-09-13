use redis::AsyncCommands;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::job_key};

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
