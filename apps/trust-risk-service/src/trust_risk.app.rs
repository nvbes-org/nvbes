use std::{sync::Arc, time::Instant};

use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::config::TrustRiskConfig;

#[derive(Clone)]
pub struct TrustRiskState {
    pub config: Arc<TrustRiskConfig>,
    pub db: PgPool,
    pub metrics: Arc<PrometheusHandle>,
    pub projection_heartbeat: Arc<RwLock<Option<Instant>>>,
}

impl TrustRiskState {
    pub fn new(config: TrustRiskConfig, db: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            db,
            metrics: crate::risk_metrics::install(),
            projection_heartbeat: Arc::new(RwLock::new(None)),
        }
    }
}
