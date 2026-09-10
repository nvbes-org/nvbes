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

    pub async fn connect_runtime(config: &AppConfig) -> CloudResult<Self> {
        Self::connect_with_role(config, "NVBES_CLOUD_RUNTIME_DATABASE_URL", "nvbes_app").await
    }

    pub async fn connect_system(config: &AppConfig) -> CloudResult<Self> {
        Self::connect_with_role(config, "NVBES_CLOUD_SYSTEM_DATABASE_URL", "nvbes_system").await
    }

    async fn connect_with_role(
        config: &AppConfig,
        environment_variable: &str,
        database_role: &str,
    ) -> CloudResult<Self> {
        let database_url = match std::env::var(environment_variable) {
            Ok(value) if !value.trim().is_empty() => value,
            _ if config.environment == "development" => config.database_url.clone(),
            _ => {
                return Err(crate::CloudError::Configuration(format!(
                    "{environment_variable} is required outside development"
                )));
            }
        };
        let settings = nvbes_core::postgres_runtime::PostgresPoolSettings::from_app_config(config);
        let pool = nvbes_core::postgres_runtime::connect_pool_with_url_and_role(
            settings,
            &database_url,
            database_role,
        )
        .await?;

        let active_role: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&pool)
            .await?;
        if active_role != database_role {
            return Err(crate::CloudError::Configuration(format!(
                "expected database role {database_role}, got {active_role}"
            )));
        }

        Ok(Self { pool })
    }
}

impl std::ops::Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
