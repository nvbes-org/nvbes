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
    let cors = state.config.browser_origins.layer(vec![
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
    ]);
    let browser = crate::profile::router(state.clone())
        .merge(crate::preferences::router(state.clone()))
        .merge(crate::teams::router(state.clone()))
        .merge(crate::privacy::router(state.clone()))
        .layer(cors);
    crate::health::router(state.clone())
        .merge(crate::billing_authorization::router(
            state.db.clone(),
            state.config.billing_authorization_secret.as_deref(),
        ))
        .merge(crate::metrics::router(state))
        .merge(browser)
}
