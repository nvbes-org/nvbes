use nvbes_core::config::AppConfig;
use sqlx::postgres::PgConnectOptions;
use std::str::FromStr;

const MIGRATION_DATABASE_URL_ENV: &str = "NVBES_ACCOUNT_MIGRATION_DATABASE_URL";

pub async fn run(config: &AppConfig) -> anyhow::Result<()> {
    let migration_database_url = resolve_migration_database_url(
        &config.environment,
        &config.database_url,
        std::env::var(MIGRATION_DATABASE_URL_ENV).ok(),
    )?;
    let migration_config = AppConfig {
        database_url: migration_database_url,
        ..config.clone()
    };
    let db = nvbes_core::postgres_runtime::connect_pool(&migration_config).await?;
    sqlx::migrate!("../account-service/migrations")
        .run(&db)
        .await?;
    db.close().await;
    Ok(())
}

fn resolve_migration_database_url(
    environment: &str,
    runtime_database_url: &str,
    configured_database_url: Option<String>,
) -> anyhow::Result<String> {
    let configured_database_url = configured_database_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if environment == "development" {
        return Ok(configured_database_url.unwrap_or_else(|| runtime_database_url.to_string()));
    }

    let migration_database_url = configured_database_url.ok_or_else(|| {
        anyhow::anyhow!("{MIGRATION_DATABASE_URL_ENV} must be configured outside development")
    })?;
    let runtime_options = PgConnectOptions::from_str(runtime_database_url)?;
    let migration_options = PgConnectOptions::from_str(&migration_database_url)?;
    if migration_options.get_username() == runtime_options.get_username() {
        anyhow::bail!(
            "{MIGRATION_DATABASE_URL_ENV} must use a database role distinct from the runtime role"
        );
    }
    Ok(migration_database_url)
}

#[cfg(test)]
mod tests {
    use super::resolve_migration_database_url;

    #[test]
    fn production_requires_a_dedicated_migration_database_url() {
        let error =
            resolve_migration_database_url("production", "postgres://runtime@db/account", None)
                .expect_err("missing migration URL should fail");

        assert!(error.to_string().contains("must be configured"));
    }

    #[test]
    fn production_rejects_the_runtime_database_url() {
        let runtime = "postgres://runtime@db/account";
        let error =
            resolve_migration_database_url("production", runtime, Some(runtime.to_string()))
                .expect_err("runtime credentials should not migrate");

        assert!(error.to_string().contains("distinct"));
    }

    #[test]
    fn production_rejects_the_runtime_role_even_when_the_url_differs() {
        let error = resolve_migration_database_url(
            "production",
            "postgres://runtime@db/account",
            Some("postgres://runtime@db/account?application_name=migrator".to_string()),
        )
        .expect_err("runtime role should not migrate");

        assert!(error.to_string().contains("distinct"));
    }

    #[test]
    fn development_can_reuse_the_runtime_database_url() {
        let runtime = "postgres://postgres@localhost/account";

        assert_eq!(
            resolve_migration_database_url("development", runtime, None)
                .expect("development should use its runtime database"),
            runtime
        );
    }
}
