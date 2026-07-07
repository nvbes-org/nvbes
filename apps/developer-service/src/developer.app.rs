use nvbes_core::config::AppConfig;
use sqlx::PgPool;

#[derive(Clone)]
pub struct DeveloperAppState {
    pub config: AppConfig,
    pub db: PgPool,
}

impl DeveloperAppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            db,
        })
    }
}
