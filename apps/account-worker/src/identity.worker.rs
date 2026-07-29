#[path = "identity.worker.audit_anchor.rs"]
pub mod audit_anchor;
#[path = "identity.worker.enterprise_grpc.rs"]
pub mod enterprise_grpc;
#[path = "identity.worker.housekeeping.rs"]
pub mod housekeeping;
#[path = "identity.worker.jobs.rs"]
pub mod jobs;
#[path = "identity.worker.loop.rs"]
pub mod loop_;
#[path = "identity.worker.queue.metrics.rs"]
pub mod queue_metrics;

pub use loop_::run_loop_until_shutdown;

pub async fn run_housekeeping_job(state: &crate::app::AppState) -> anyhow::Result<()> {
    housekeeping::run_once(state).await
}
