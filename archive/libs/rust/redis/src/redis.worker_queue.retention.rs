use serde_json::json;

use super::{QueuedJob, keys::*};

pub(super) const TERMINAL_RETENTION_ENV: &str = "NVBES_WORKER_QUEUE_TERMINAL_RETENTION_SECONDS";
pub(super) const DEFAULT_TERMINAL_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;
pub(super) const MIN_TERMINAL_RETENTION_SECONDS: u64 = 60;
pub(super) const MAX_TERMINAL_RETENTION_SECONDS: u64 = 30 * 24 * 60 * 60;
pub(super) const REDACTED_TERMINAL_ERROR: &str =
    "job execution failed; details are available in protected observability";

pub(super) fn terminal_retention_seconds() -> u64 {
    bounded_terminal_retention_seconds(std::env::var(TERMINAL_RETENTION_ENV).ok().as_deref())
}

pub(super) fn bounded_terminal_retention_seconds(configured: Option<&str>) -> u64 {
    configured
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TERMINAL_RETENTION_SECONDS)
        .clamp(
            MIN_TERMINAL_RETENTION_SECONDS,
            MAX_TERMINAL_RETENTION_SECONDS,
        )
}

pub(super) fn terminal_expiry(now: i64, retention_seconds: u64) -> i64 {
    now.saturating_add(i64::try_from(retention_seconds).unwrap_or(i64::MAX))
}

pub(super) fn succeeded_snapshot(job: &QueuedJob, now: i64) -> QueuedJob {
    let mut snapshot = terminal_snapshot(job, now);
    snapshot.status = STATUS_SUCCEEDED.to_string();
    snapshot.last_error = None;
    snapshot.result = Some(json!({ "redacted": true }));
    snapshot
}

pub(super) fn dead_letter_snapshot(job: &QueuedJob, now: i64) -> QueuedJob {
    let mut snapshot = terminal_snapshot(job, now);
    snapshot.status = STATUS_DEAD_LETTER.to_string();
    snapshot.last_error = Some(REDACTED_TERMINAL_ERROR.to_string());
    snapshot.result = None;
    snapshot
}

pub(super) fn existing_terminal_snapshot(job: &QueuedJob) -> Option<QueuedJob> {
    match job.status.as_str() {
        STATUS_SUCCEEDED => Some(succeeded_snapshot(job, job.updated_at)),
        STATUS_DEAD_LETTER => Some(dead_letter_snapshot(job, job.updated_at)),
        _ => None,
    }
}

fn terminal_snapshot(job: &QueuedJob, now: i64) -> QueuedJob {
    let mut snapshot = job.clone();
    snapshot.idempotency_key = None;
    snapshot.payload = json!({});
    snapshot.updated_at = now;
    snapshot.claimed_at = None;
    snapshot.lease_token = None;
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_configuration_uses_a_safe_default_and_bounds() {
        assert_eq!(
            bounded_terminal_retention_seconds(None),
            DEFAULT_TERMINAL_RETENTION_SECONDS
        );
        assert_eq!(
            bounded_terminal_retention_seconds(Some("invalid")),
            DEFAULT_TERMINAL_RETENTION_SECONDS
        );
        assert_eq!(
            bounded_terminal_retention_seconds(Some("0")),
            MIN_TERMINAL_RETENTION_SECONDS
        );
        assert_eq!(
            bounded_terminal_retention_seconds(Some("999999999")),
            MAX_TERMINAL_RETENTION_SECONDS
        );
        assert_eq!(bounded_terminal_retention_seconds(Some(" 3600 ")), 60 * 60);
    }

    #[test]
    fn terminal_expiry_saturates_instead_of_overflowing() {
        assert_eq!(terminal_expiry(i64::MAX, u64::MAX), i64::MAX);
    }
}
