use redis::AsyncCommands;

use super::{QueuedJob, keys::queue_key};
use crate::connection::{RedisError, RedisPool};

pub async fn find_latest_job<F>(
    pool: &RedisPool,
    queue: &str,
    mut matches: F,
) -> Result<Option<QueuedJob>, RedisError>
where
    F: FnMut(&QueuedJob) -> bool,
{
    let mut conn = pool.get().await?;
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(queue_key(queue, "job:*"))
        .query_async(&mut *conn)
        .await?;

    let mut latest: Option<QueuedJob> = None;
    for key in job_keys {
        let raw: String = match conn.get(&key).await {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let Ok(job) = serde_json::from_str::<QueuedJob>(&raw) else {
            continue;
        };
        if !matches(&job) {
            continue;
        }

        match latest {
            Some(ref current) if current.created_at >= job.created_at => {}
            _ => latest = Some(job),
        }
    }

    Ok(latest)
}

pub async fn find_matching_jobs<F>(
    pool: &RedisPool,
    queue: &str,
    mut matches: F,
) -> Result<Vec<QueuedJob>, RedisError>
where
    F: FnMut(&QueuedJob) -> bool,
{
    let mut conn = pool.get().await?;
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(queue_key(queue, "job:*"))
        .query_async(&mut *conn)
        .await?;

    let mut jobs = Vec::new();
    for key in job_keys {
        let raw: String = match conn.get(&key).await {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let Ok(job) = serde_json::from_str::<QueuedJob>(&raw) else {
            continue;
        };
        if matches(&job) {
            jobs.push(job);
        }
    }

    jobs.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| right.id.cmp(&left.id))
    });

    Ok(jobs)
}
