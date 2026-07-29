use super::QueuedJob;

pub(super) const KEY_PREFIX: &str = "nvbes:worker_queue";
pub(super) const STATUS_PENDING: &str = "pending";
pub(super) const STATUS_RUNNING: &str = "running";
pub(super) const STATUS_FAILED: &str = "failed";
pub(super) const STATUS_DEAD_LETTER: &str = "dead_letter";
pub(super) const STATUS_SUCCEEDED: &str = "succeeded";

pub(super) fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

pub(super) fn queue_key(queue: &str, suffix: &str) -> String {
    format!("{KEY_PREFIX}:{queue}:{suffix}")
}

pub(super) fn job_key(queue: &str, job_id: uuid::Uuid) -> String {
    queue_key(queue, &format!("job:{job_id}"))
}

pub(super) fn lease_key(queue: &str, job_id: uuid::Uuid) -> String {
    queue_key(queue, &format!("lease:{job_id}"))
}

pub(super) fn dedupe_key(queue: &str, job_type: &str, idempotency_key: &str) -> String {
    queue_key(queue, &format!("dedupe:{job_type}:{idempotency_key}"))
}

pub(super) fn pending_key(queue: &str) -> String {
    queue_key(queue, "pending")
}

pub(super) fn running_key(queue: &str) -> String {
    queue_key(queue, "running")
}

pub(super) fn delayed_key(queue: &str) -> String {
    queue_key(queue, "delayed")
}

pub(super) fn dead_letter_key(queue: &str) -> String {
    queue_key(queue, "dead_letter")
}

pub(super) fn job_age_seconds(job: &QueuedJob, status: &str) -> Option<f64> {
    let origin = match status {
        STATUS_RUNNING => job.claimed_at,
        STATUS_FAILED => Some(job.available_at),
        STATUS_PENDING => Some(job.created_at),
        STATUS_DEAD_LETTER | STATUS_SUCCEEDED => Some(job.updated_at),
        _ => None,
    }?;
    Some((now_ts() - origin).max(0) as f64)
}
