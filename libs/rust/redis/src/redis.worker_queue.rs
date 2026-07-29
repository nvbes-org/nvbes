use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[path = "redis.worker_queue.claim.rs"]
mod claim;
#[path = "redis.worker_queue.completion.rs"]
mod completion;
#[path = "redis.worker_queue.enqueue.rs"]
mod enqueue;
#[path = "redis.worker_queue.find.rs"]
mod find;
#[path = "redis.worker_queue.keys.rs"]
mod keys;
#[path = "redis.worker_queue.lease.rs"]
mod lease;
#[cfg(test)]
#[path = "redis.worker_queue.lease.tests.rs"]
mod lease_tests;
#[path = "redis.worker_queue.promotion.rs"]
mod promotion;
#[path = "redis.worker_queue.recovery.rs"]
mod recovery;
#[cfg(test)]
#[path = "redis.worker_queue.reliability.tests.rs"]
mod reliability_tests;
#[path = "redis.worker_queue.retention.rs"]
mod retention;
#[path = "redis.worker_queue.retention.migration.rs"]
mod retention_migration;
#[cfg(test)]
#[path = "redis.worker_queue.retention.migration.tests.rs"]
mod retention_migration_tests;
#[cfg(test)]
#[path = "redis.worker_queue.retention.tests.rs"]
mod retention_tests;
#[path = "redis.worker_queue.status.rs"]
mod status;
#[cfg(test)]
#[path = "redis.worker_queue.status.tests.rs"]
mod status_tests;
#[path = "redis.worker_queue.store.rs"]
mod store;
#[cfg(test)]
#[path = "redis.worker_queue.test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "redis.worker_queue.tests.rs"]
mod tests;
#[cfg(test)]
#[path = "redis.worker_queue.volume.tests.rs"]
mod volume_tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedJob {
    pub id: Uuid,
    pub queue: String,
    pub job_type: String,
    pub idempotency_key: Option<String>,
    pub payload: Value,
    pub max_attempts: u32,
    pub attempts: u32,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub claimed_at: Option<i64>,
    #[serde(default)]
    pub lease_token: Option<Uuid>,
    pub available_at: i64,
    pub last_error: Option<String>,
    pub result: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct QueueStatusEntry {
    pub status: String,
    pub depth: i64,
    pub oldest_age_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct EnqueueJobInput {
    pub queue: String,
    pub job_type: String,
    pub payload: Value,
    pub idempotency_key: Option<String>,
    pub max_attempts: u32,
    pub overwrite_terminal: bool,
    pub job_id: Option<Uuid>,
}

pub use claim::claim_next_job;
pub use completion::{mark_job_failed, mark_job_succeeded};
pub use enqueue::enqueue_job;
pub use find::{find_latest_job, find_matching_jobs};
pub use lease::renew_job_lease;
pub use promotion::promote_due_jobs;
pub use recovery::recover_stale_jobs;
pub use status::queue_status;
pub use store::get_job;
