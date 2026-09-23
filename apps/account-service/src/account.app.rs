use std::sync::Arc;

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;

use crate::{auth::TokenVerifier, config::AccountConfig};

#[derive(Clone)]
pub struct AccountState {
    pub config: Arc<AccountConfig>,
    pub db: PgPool,
    pub metrics: Arc<PrometheusHandle>,
    pub tokens: Arc<TokenVerifier>,
}

impl AccountState {
    pub fn new(config: AccountConfig, db: PgPool) -> anyhow::Result<Self> {
        let tokens = TokenVerifier::new(&config)?;
        Ok(Self {
            config: Arc::new(config),
            db,
            metrics: crate::metrics::install(),
            tokens: Arc::new(tokens),
        })
    }
}

pub fn router(state: AccountState) -> Router {
    crate::health::router(state.clone())
        .merge(crate::metrics::router(state.clone()))
        .merge(crate::profile::router(state.clone()))
        .merge(crate::preferences::router(state.clone()))
        .merge(crate::consents::router(state.clone()))
        .merge(crate::teams::router(state.clone()))
        .merge(crate::privacy::router(state))
}
