use anyhow::Context;
use sqlx::PgPool;

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("ensuring multi-schema isolation in PostgreSQL");

    let schemas = ["identity", "account", "billing", "consent", "outbox"];
    for schema in schemas {
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}")) // nvbes-allow-runtime-sql
            .execute(pool)
            .await
            .with_context(|| format!("failed to create schema {schema}"))?;
    }

    tracing::info!("running Identity domain migrations");
    {
        let mut conn = pool
            .acquire()
            .await
            .context("failed to acquire connection for identity migrations")?;
        sqlx::query("SET search_path TO identity, public") // nvbes-allow-runtime-sql
            .execute(&mut *conn)
            .await
            .context("failed to set identity search_path")?;
        nvbes_identity_service::migrations::MIGRATOR
            .run(&mut *conn)
            .await
            .context("failed to run identity migrations")?;
    }

    tracing::info!("running Account domain migrations");
    {
        let mut conn = pool
            .acquire()
            .await
            .context("failed to acquire connection for account migrations")?;
        sqlx::query("SET search_path TO account, public") // nvbes-allow-runtime-sql
            .execute(&mut *conn)
            .await
            .context("failed to set account search_path")?;
        nvbes_account_service::database::MIGRATOR
            .run(&mut *conn)
            .await
            .context("failed to run account migrations")?;
    }

    tracing::info!("running Billing domain migrations");
    {
        let mut conn = pool
            .acquire()
            .await
            .context("failed to acquire connection for billing migrations")?;
        sqlx::query("SET search_path TO billing, public") // nvbes-allow-runtime-sql
            .execute(&mut *conn)
            .await
            .context("failed to set billing search_path")?;
        nvbes_billing_service::database::MIGRATOR
            .run(&mut *conn)
            .await
            .context("failed to run billing migrations")?;
    }

    tracing::info!("running Consent domain migrations");
    {
        let mut conn = pool
            .acquire()
            .await
            .context("failed to acquire connection for consent migrations")?;
        sqlx::query("SET search_path TO consent, public") // nvbes-allow-runtime-sql
            .execute(&mut *conn)
            .await
            .context("failed to set consent search_path")?;
        nvbes_consent::db::MIGRATOR
            .run(&mut *conn)
            .await
            .context("failed to run consent migrations")?;
    }

    tracing::info!("all domain migrations applied successfully");
    Ok(())
}
