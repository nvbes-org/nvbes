use anyhow::Context;
use sqlx::{
    PgPool,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
};

/// Each domain migrates inside its own schema with its own migration table, so
/// the domains stay separable without coordinating version numbers.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let options = pool.connect_options().as_ref().clone();
    for (schema, migrator) in [
        ("identity", &nvbes_identity_service::migrations::MIGRATOR),
        ("account", &nvbes_account_service::database::MIGRATOR),
    ] {
        migrate_schema(options.clone(), schema, migrator)
            .await
            .with_context(|| format!("failed to migrate the {schema} schema"))?;
    }
    tracing::info!("all domain migrations applied");
    Ok(())
}

async fn migrate_schema(
    options: PgConnectOptions,
    schema: &'static str,
    migrator: &Migrator,
) -> anyhow::Result<()> {
    tracing::info!(schema, "running domain migrations");
    let schema_pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _| {
            Box::pin(async move {
                sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}")) // nvbes-allow-runtime-sql
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SELECT set_config('search_path', $1, false)") // nvbes-allow-runtime-sql
                    .bind(format!("{schema}, public"))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await?;
    let result = migrator.run(&schema_pool).await;
    schema_pool.close().await;
    Ok(result?)
}
