use redis::AsyncCommands;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{QueueStatusEntry, keys::*, store::load_job};

pub async fn queue_status(
    pool: &RedisPool,
    queue: &str,
) -> Result<Vec<QueueStatusEntry>, RedisError> {
    let mut conn = pool.get().await?;
    let pending_depth: i64 = conn.llen(pending_key(queue)).await?;
    let running_depth: i64 = conn.zcard(running_key(queue)).await?;
    let failed_depth: i64 = conn.zcard(delayed_key(queue)).await?;
    let dead_letter_depth: i64 = conn.scard(dead_letter_key(queue)).await?;

    let mut statuses = Vec::new();
    if pending_depth > 0 {
        let oldest_id: Option<String> = conn.lindex(pending_key(queue), 0).await?;
        let oldest_age_seconds = if let Some(id) = oldest_id {
            if let Ok(job_id) = Uuid::parse_str(&id) {
                load_job(pool, queue, job_id)
                    .await
                    .ok()
                    .and_then(|job| job_age_seconds(&job, STATUS_PENDING))
            } else {
                None
            }
        } else {
            None
        };
        statuses.push(QueueStatusEntry {
            status: STATUS_PENDING.to_string(),
            depth: pending_depth,
            oldest_age_seconds,
        });
    }

    if running_depth > 0 {
        let oldest: Vec<String> = redis::cmd("ZRANGE")
            .arg(running_key(queue))
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();
        let oldest_age_seconds = oldest
            .get(1)
            .and_then(|score| score.parse::<f64>().ok())
            .map(|score| (now_ts() as f64 - score).max(0.0));
        statuses.push(QueueStatusEntry {
            status: STATUS_RUNNING.to_string(),
            depth: running_depth,
            oldest_age_seconds,
        });
    }

    if failed_depth > 0 {
        let oldest: Vec<String> = redis::cmd("ZRANGE")
            .arg(delayed_key(queue))
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();
        let oldest_age_seconds = oldest
            .get(1)
            .and_then(|score| score.parse::<f64>().ok())
            .map(|score| (now_ts() as f64 - score).max(0.0));
        statuses.push(QueueStatusEntry {
            status: STATUS_FAILED.to_string(),
            depth: failed_depth,
            oldest_age_seconds,
        });
    }

    if dead_letter_depth > 0 {
        statuses.push(QueueStatusEntry {
            status: STATUS_DEAD_LETTER.to_string(),
            depth: dead_letter_depth,
            oldest_age_seconds: None,
        });
    }

    Ok(statuses)
}
