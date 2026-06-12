use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(config: &AppConfig) -> anyhow::Result<Self> {
        let pool = nvbes_core::postgres_runtime::connect_pool(config).await?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;

        Ok(())
    }
}

impl std::ops::Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
