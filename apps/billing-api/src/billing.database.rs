use nvbes_core::{
    config::AppConfig,
    postgres_runtime::{PostgresPoolSettings, connect_pool_with_url},
};
use sqlx::PgPool;

pub type Database = PgPool;

pub async fn connect_pool(config: &AppConfig) -> Result<Database, sqlx::Error> {
    let settings = PostgresPoolSettings::from_app_config(config);
    connect_pool_with_url(settings, config.billing_database_url()).await
}

pub async fn run_migrations(pool: &Database) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
