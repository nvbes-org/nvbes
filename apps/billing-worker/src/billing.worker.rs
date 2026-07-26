use std::sync::Arc;

#[path = "billing.worker.analytics.rs"]
pub mod analytics;
#[path = "billing.worker.email.rs"]
pub mod email;
#[path = "billing.worker.jobs.rs"]
pub mod jobs;
#[path = "billing.worker.loop.rs"]
mod loop_;
#[path = "billing.worker.mollie.rs"]
pub mod mollie;
#[path = "billing.worker.workspace_updates.rs"]
pub mod workspace_updates;

pub use loop_::{run_billing_jobs_once, run_loop_until_shutdown};

#[derive(Clone)]
pub struct BillingWorkerState {
    pub config: nvbes_core::config::AppConfig,
    pub db: sqlx::PgPool,
    pub redis: nvbes_redis::RedisPool,
    pub email: Arc<dyn nvbes_email::EmailSender>,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub product_analytics: nvbes_product_analytics::ProductAnalytics,
}

impl BillingWorkerState {
    pub fn new(
        config: nvbes_core::config::AppConfig,
        db: sqlx::PgPool,
        redis: nvbes_redis::RedisPool,
        email: Arc<dyn nvbes_email::EmailSender>,
        product_analytics: nvbes_product_analytics::ProductAnalytics,
    ) -> Self {
        Self {
            config,
            db,
            redis,
            email,
            observability: nvbes_observability::metrics::HttpMetrics::default(),
            product_analytics,
        }
    }
}
