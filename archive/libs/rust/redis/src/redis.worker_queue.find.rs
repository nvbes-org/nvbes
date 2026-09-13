use super::{QueuedJob, keys::queue_key};
use crate::connection::{RedisError, RedisPool};

const SCAN_BATCH_SIZE: usize = 250;

pub async fn find_latest_job<F>(
    pool: &RedisPool,
    queue: &str,
    mut matches: F,
) -> Result<Option<QueuedJob>, RedisError>
where
    F: FnMut(&QueuedJob) -> bool,
{
    let mut latest: Option<QueuedJob> = None;
    for job in scan_jobs(pool, queue).await? {
        if !matches(&job) {
            continue;
        }

        match latest {
            Some(ref current)
                if (current.created_at, current.updated_at, current.id)
                    >= (job.created_at, job.updated_at, job.id) => {}
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
    let mut jobs = Vec::new();
    for job in scan_jobs(pool, queue).await? {
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

async fn scan_jobs(pool: &RedisPool, queue: &str) -> Result<Vec<QueuedJob>, RedisError> {
    let mut conn = pool.get().await?;
    let pattern = queue_key(queue, "job:*");
    let mut cursor = 0_u64;
    let mut jobs = Vec::new();
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(SCAN_BATCH_SIZE)
            .query_async(&mut *conn)
            .await?;
        if !keys.is_empty() {
            let values: Vec<Option<String>> = redis::cmd("MGET")
                .arg(&keys)
                .query_async(&mut *conn)
                .await?;
            jobs.extend(
                values
                    .into_iter()
                    .flatten()
                    .filter_map(|raw| serde_json::from_str(&raw).ok()),
            );
        }
        if next_cursor == 0 {
            return Ok(jobs);
        }
        cursor = next_cursor;
    }
}
