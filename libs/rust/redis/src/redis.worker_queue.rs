use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[path = "redis.worker_queue.find.rs"]
mod find;
#[path = "redis.worker_queue.keys.rs"]
mod keys;
#[path = "redis.worker_queue.lifecycle.rs"]
mod lifecycle;
#[path = "redis.worker_queue.status.rs"]
mod status;
#[path = "redis.worker_queue.store.rs"]
mod store;
#[cfg(test)]
#[path = "redis.worker_queue.tests.rs"]
mod tests;

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

pub use find::{find_latest_job, find_matching_jobs};
pub use lifecycle::{
    claim_next_job, enqueue_job, mark_job_failed, mark_job_succeeded, promote_due_jobs,
    recover_stale_jobs,
};
pub use status::queue_status;
pub use store::get_job;
