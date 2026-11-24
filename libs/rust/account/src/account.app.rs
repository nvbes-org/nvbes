use std::sync::Arc;

use axum::Router;
use nvbes_identity_sdk::introspection::ActivityVerifier;
use sqlx::PgPool;

use crate::{
    auth::{TokenTrust, TokenVerifier},
    config::AccountConfig,
};

#[derive(Clone)]
pub struct AccountState {
    pub config: Arc<AccountConfig>,
    pub trust: Arc<TokenTrust>,
    pub db: PgPool,
    pub tokens: Arc<TokenVerifier>,
}

impl AccountState {
    pub fn new(
        config: AccountConfig,
        trust: TokenTrust,
        db: PgPool,
        activity: Arc<dyn ActivityVerifier>,
    ) -> anyhow::Result<Self> {
        let tokens = TokenVerifier::new(&trust, config.public_origin.as_deref(), activity)?;
        Ok(Self {
            config: Arc::new(config),
            trust: Arc::new(trust),
            db,
            tokens: Arc::new(tokens),
        })
    }
}

pub fn domain_router(state: AccountState) -> Router {
    let cors = state.config.browser_origins.layer(vec![
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
    ]);
    let browser = crate::profile::router(state.clone())
        .merge(crate::preferences::router(state.clone()))
        .merge(crate::consents::router(state.clone()))
        .merge(crate::privacy::router(state.clone()))
        .layer(cors);
    crate::operator::router(state.clone()).merge(browser)
}
