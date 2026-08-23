use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use crate::identity::IdentityClient;

#[derive(Clone)]
pub struct DeveloperAppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub identity: IdentityClient,
}

impl DeveloperAppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            db,
            identity: IdentityClient::from_env()?,
        })
    }
}
