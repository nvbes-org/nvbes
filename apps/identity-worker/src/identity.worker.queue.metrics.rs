use std::time::{Duration, Instant};

use crate::app::AppState;

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(15);
const QUEUE_STATUSES: [&str; 4] = ["pending", "running", "failed", "dead_letter"];

pub async fn refresh_if_due(
    state: &AppState,
    queues: &[&str],
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < REFRESH_INTERVAL {
        return Ok(());
    }

    for queue in queues {
        let entries = nvbes_redis::worker_queue::queue_status(&state.redis, queue).await?;
        for status in QUEUE_STATUSES {
            let entry = entries.iter().find(|entry| entry.status == status);
            state.observability.record_worker_queue_depth(
                queue,
                status,
                entry.map_or(0, |entry| entry.depth),
                entry.and_then(|entry| entry.oldest_age_seconds),
            );
        }
    }

    *last_run = Instant::now();
    Ok(())
}
