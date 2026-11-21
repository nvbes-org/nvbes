use std::sync::Arc;

use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;

use crate::config::IdentityConfig;

#[derive(Clone)]
pub struct IdentityState {
    pub config: Arc<IdentityConfig>,
    pub db: PgPool,
    pub metrics: Arc<PrometheusHandle>,
}

impl IdentityState {
    pub fn new(config: IdentityConfig, db: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            db,
            metrics: crate::metrics::install(),
        }
    }
}
