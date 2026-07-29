#[path = "identity.database.migrations.tests.cluster.rs"]
mod cluster;
#[path = "identity.database.migrations.tests.support.rs"]
mod support;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use sqlx::migrate::Migrator;
use uuid::Uuid;

use self::support::{
    EphemeralPostgresDatabase, assert_named_resources_absent, cleanup_current_run,
    cluster_role_names, migration_admin_url,
};

static ACCOUNT_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[test]
fn migration_versions_are_unique() {
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut migrations_by_version = BTreeMap::<u64, Vec<String>>::new();

    for entry in fs::read_dir(&migrations_dir).expect("account migrations directory must exist") {
        let path = entry
            .expect("migration directory entry must be readable")
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("sql") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("migration filename must be valid UTF-8");
        let version = file_name
            .split_once('_')
            .expect("migration filename must start with a numeric version")
            .0
            .parse::<u64>()
            .expect("migration version must be numeric");

        migrations_by_version
            .entry(version)
            .or_default()
            .push(file_name.to_string());
    }

    let duplicates = migrations_by_version
        .into_iter()
        .filter(|(_, filenames)| filenames.len() > 1)
        .collect::<Vec<_>>();

    assert!(
        duplicates.is_empty(),
        "SQLx migration versions must be unique; duplicates: {duplicates:?}"
    );
}

#[tokio::test]
async fn postgresql_fresh_database_reaches_complete_schema() -> Result<()> {
    let admin_url = migration_admin_url()?;
    let roles_before = cluster_role_names(&admin_url).await?;
    let database = EphemeralPostgresDatabase::create(&admin_url, "fresh").await?;
    let database_name = database.name().to_owned();
    let pool = database.connect().await?;

    ACCOUNT_MIGRATOR.run(&pool).await?;
    assert_all_migrations_applied(&pool).await?;

    for relation in ["principals", "email_messages", "oauth_client_keys"] {
        let qualified_name = format!("public.{relation}");
        let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(&qualified_name)
            .fetch_one(&pool)
            .await?;
        assert!(exists, "fresh schema is missing {qualified_name}");
    }

    let job_id = Uuid::new_v4();
    let (message_id, attempt_count): (String, i32) = sqlx::query_as(
        r#"
        INSERT INTO email_messages (
            job_id, business_type, recipient_email, recipient_hash
        )
        VALUES ($1, 'migration_fresh', 'fresh@example.test', 'fresh-hash')
        RETURNING message_id, attempt_count
        "#,
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        message_id,
        format!("<account-job-{job_id}@worker.nvbes.fr>")
    );
    assert_eq!(attempt_count, 0);

    pool.close().await;
    database.cleanup().await?;
    assert_named_resources_absent(&admin_url, &database_name).await?;
    assert_eq!(
        cluster_role_names(&admin_url).await?,
        roles_before,
        "fresh migration must leave zero new cluster-wide roles"
    );
    Ok(())
}

#[tokio::test]
async fn postgresql_upgrade_from_n_minus_one_preserves_compatible_data() -> Result<()> {
    let admin_url = migration_admin_url()?;
    let roles_before = cluster_role_names(&admin_url).await?;
    let database = EphemeralPostgresDatabase::create(&admin_url, "upgrade").await?;
    let database_name = database.name().to_owned();
    let pool = database.connect().await?;
    let (prior_migrator, latest_version) = prior_migrator()?;

    prior_migrator.run(&pool).await?;
    let old_job_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id, business_type, recipient_email, recipient_hash
        )
        VALUES ($1, 'migration_upgrade', 'upgrade@example.test', 'upgrade-hash')
        "#,
    )
    .bind(old_job_id)
    .execute(&pool)
    .await
    .context("seed an N-1 email row")?;

    ACCOUNT_MIGRATOR.run(&pool).await?;
    ACCOUNT_MIGRATOR.run(&pool).await?;
    assert_all_migrations_applied(&pool).await?;

    let applied_latest: i64 =
        sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(&pool)
            .await?;
    assert_eq!(applied_latest, latest_version);

    let (message_id, attempt_count, created, status): (String, i32, bool, String) = sqlx::query_as(
        r#"
            SELECT message_id, attempt_count, created_at IS NOT NULL, status::text
            FROM email_messages
            WHERE job_id = $1
            "#,
    )
    .bind(old_job_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        message_id,
        format!("<account-job-{old_job_id}@worker.nvbes.fr>")
    );
    assert_eq!(
        (attempt_count, created, status.as_str()),
        (0, true, "queued")
    );

    let compatible_job_id = Uuid::new_v4();
    let compatible_message_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO email_messages (
            job_id, business_type, recipient_email, recipient_hash
        )
        VALUES ($1, 'n_minus_one_writer', 'old-writer@example.test', 'old-writer-hash')
        RETURNING message_id
        "#,
    )
    .bind(compatible_job_id)
    .fetch_one(&pool)
    .await
    .context("the N-1 insert shape must remain valid after upgrade")?;
    assert_eq!(
        compatible_message_id,
        format!("<account-job-{compatible_job_id}@worker.nvbes.fr>")
    );

    pool.close().await;
    database.cleanup().await?;
    assert_named_resources_absent(&admin_url, &database_name).await?;
    assert_eq!(
        cluster_role_names(&admin_url).await?,
        roles_before,
        "N-1 upgrade must leave zero new cluster-wide roles"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "invoked by scripts/test-account-service-migrations.sh as an EXIT cleanup"]
async fn cleanup_ephemeral_databases_for_current_run() -> Result<()> {
    cleanup_current_run(&migration_admin_url()?).await
}

#[tokio::test]
#[ignore = "script fixture that simulates process termination before RAII cleanup"]
async fn prepare_interrupted_cluster_state_for_recovery_test() -> Result<()> {
    let admin_url = migration_admin_url()?;
    let database = EphemeralPostgresDatabase::create(&admin_url, "recovery").await?;
    let pool = database.connect().await?;
    ACCOUNT_MIGRATOR.run(&pool).await?;
    pool.close().await;

    std::mem::forget(database);
    Ok(())
}

fn prior_migrator() -> Result<(Migrator, i64)> {
    let up_migrations = ACCOUNT_MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .cloned()
        .collect::<Vec<_>>();
    let latest = up_migrations
        .last()
        .context("at least one Account migration is required")?;
    if up_migrations.len() < 2 {
        anyhow::bail!("an N-1 migration set requires at least two migrations");
    }

    let prior_migrator = Migrator {
        migrations: Cow::Owned(up_migrations[..up_migrations.len() - 1].to_vec()),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    Ok((prior_migrator, latest.version))
}

async fn assert_all_migrations_applied(pool: &sqlx::PgPool) -> Result<()> {
    let expected = ACCOUNT_MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .count() as i64;
    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(pool)
        .await?;
    assert_eq!(applied, expected, "every Account migration must be applied");
    Ok(())
}
