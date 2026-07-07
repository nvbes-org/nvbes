use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;

use crate::CloudResult;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(config: &AppConfig) -> CloudResult<Self> {
        let pool = nvbes_core::postgres_runtime::connect_pool(config).await?;

        Ok(Self { pool })
    }
}

impl std::ops::Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
