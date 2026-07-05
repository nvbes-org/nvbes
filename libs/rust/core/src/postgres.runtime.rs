use std::time::Duration;

use crate::config::AppConfig;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Connection, Executor};
use tracing::{debug, warn};

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const IDLE_PING_AFTER: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostgresPoolSettings {
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    pub max_lifetime: Duration,
    pub idle_timeout: Duration,
}

impl PostgresPoolSettings {
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self::from_max_connections(config.database_max_connections)
    }

    pub fn from_max_connections(max_connections: u32) -> Self {
        Self {
            max_connections,
            acquire_timeout: ACQUIRE_TIMEOUT,
            max_lifetime: MAX_LIFETIME,
            idle_timeout: IDLE_TIMEOUT,
        }
    }
}

pub fn pool_options(settings: PostgresPoolSettings) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(settings.acquire_timeout)
        .max_lifetime(settings.max_lifetime)
        .idle_timeout(settings.idle_timeout)
        .test_before_acquire(true)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("SELECT 1").await?;
                debug!("Postgres connection established and validated");
                Ok(())
            })
        })
        .before_acquire(|conn, meta| {
            Box::pin(async move {
                if meta.idle_for < IDLE_PING_AFTER {
                    return Ok(true);
                }

                match conn.ping().await {
                    Ok(()) => Ok(true),
                    Err(error) => {
                        warn!(
                            %error,
                            idle_for_ms = meta.idle_for.as_millis(),
                            "Postgres idle connection failed validation; recycling"
                        );
                        Err(error)
                    }
                }
            })
        })
}

pub async fn connect_pool(config: &AppConfig) -> Result<PgPool, sqlx::Error> {
    let settings = PostgresPoolSettings::from_app_config(config);
    connect_pool_with_url(settings, &config.database_url).await
}

pub async fn connect_pool_with_url(
    settings: PostgresPoolSettings,
    database_url: &str,
) -> Result<PgPool, sqlx::Error> {
    pool_options(settings).connect(database_url).await
}

pub async fn health_check(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PostgresPoolSettings;
    use crate::config::AppConfig;
    use std::time::Duration;

    #[test]
    fn pool_settings_use_runtime_defaults_that_recycle_stale_connections() {
        let settings = PostgresPoolSettings::from_max_connections(12);

        assert_eq!(settings.max_connections, 12);
        assert_eq!(settings.acquire_timeout, Duration::from_secs(5));
        assert_eq!(settings.max_lifetime, Duration::from_secs(30 * 60));
        assert_eq!(settings.idle_timeout, Duration::from_secs(5 * 60));
    }

    #[test]
    fn pool_settings_preserve_configured_max_connections() {
        let config = AppConfig {
            database_max_connections: 23,
            ..AppConfig::default()
        };

        assert_eq!(
            PostgresPoolSettings::from_app_config(&config).max_connections,
            23
        );
    }
}
