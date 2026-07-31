use std::sync::OnceLock;
use std::thread;

use anyhow::{Context, Result, bail};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use url::Url;
use uuid::Uuid;

use super::cluster::{ClusterRoleGuard, recover_interrupted_run};

const DATABASE_PREFIX: &str = "nvbes_acct_mig_test_";

pub struct EphemeralPostgresDatabase {
    admin_url: String,
    database_name: String,
    database_url: String,
    cluster_roles: Option<ClusterRoleGuard>,
    active: bool,
}

impl EphemeralPostgresDatabase {
    pub async fn create(admin_url: &str, purpose: &str) -> Result<Self> {
        validate_safe_label(purpose, "database purpose")?;
        let mut parsed_url = validated_admin_url(admin_url)?;
        verify_admin_connection(admin_url).await?;
        let run_id = migration_run_id()?;
        let database_prefix = format!("{DATABASE_PREFIX}{run_id}_");
        let cluster_roles = ClusterRoleGuard::acquire(admin_url, run_id, &database_prefix).await?;

        let database_name = format!(
            "{DATABASE_PREFIX}{}_{}_{}",
            run_id,
            purpose,
            &Uuid::new_v4().simple().to_string()[..8]
        );
        validate_safe_label(&database_name, "database name")?;

        let mut admin = PgConnection::connect(admin_url)
            .await
            .context("connect to the PostgreSQL migration-test admin database")?;
        admin
            .execute(format!("CREATE DATABASE {database_name}").as_str())
            .await
            .context("create an ephemeral PostgreSQL migration-test database")?;
        admin.close().await?;

        parsed_url.set_path(&format!("/{database_name}"));
        Ok(Self {
            admin_url: admin_url.to_owned(),
            database_name,
            database_url: parsed_url.into(),
            cluster_roles: Some(cluster_roles),
            active: true,
        })
    }

    pub async fn connect(&self) -> Result<PgPool> {
        PgPoolOptions::new()
            .max_connections(4)
            .connect(&self.database_url)
            .await
            .context("connect to the ephemeral migration-test database")
    }

    pub fn name(&self) -> &str {
        &self.database_name
    }

    pub async fn cleanup(mut self) -> Result<()> {
        drop_database(&self.admin_url, &self.database_name).await?;
        self.cluster_roles
            .take()
            .context("migration-test cluster-role guard is missing")?
            .cleanup()
            .await?;
        self.active = false;
        Ok(())
    }
}

impl Drop for EphemeralPostgresDatabase {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let admin_url = self.admin_url.clone();
        let database_name = self.database_name.clone();
        let cluster_roles = self.cluster_roles.take();
        let cleanup = thread::Builder::new()
            .name("account-migration-db-cleanup".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .context("build migration cleanup runtime")?;
                runtime.block_on(async move {
                    drop_database(&admin_url, &database_name).await?;
                    if let Some(cluster_roles) = cluster_roles {
                        cluster_roles.cleanup().await?;
                    }
                    Result::<()>::Ok(())
                })
            });

        match cleanup {
            Ok(handle) => match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("failed to clean ephemeral migration database: {error:#}");
                }
                Err(_) => eprintln!("ephemeral migration database cleanup thread panicked"),
            },
            Err(error) => eprintln!("failed to start migration database cleanup: {error}"),
        }
    }
}

pub fn migration_admin_url() -> Result<String> {
    let value = std::env::var("IDENTITY_MIGRATION_TEST_ADMIN_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .or_else(|_| std::env::var("NVBES_IDENTITY_DATABASE_URL"))
        .context(
            "IDENTITY_MIGRATION_TEST_ADMIN_URL, DATABASE_URL, or NVBES_IDENTITY_DATABASE_URL is required",
        )?;
    validated_admin_url(&value)?;
    Ok(value)
}

pub async fn cluster_role_names(admin_url: &str) -> Result<Vec<String>> {
    let mut admin = PgConnection::connect(admin_url).await?;
    let roles = sqlx::query_scalar("SELECT rolname FROM pg_roles ORDER BY rolname")
        .fetch_all(&mut admin)
        .await?;
    admin.close().await?;
    Ok(roles)
}

pub async fn cleanup_current_run(admin_url: &str) -> Result<()> {
    let prefix = format!("{DATABASE_PREFIX}{}_", migration_run_id()?);
    recover_interrupted_run(admin_url, migration_run_id()?, &prefix).await
}

pub async fn assert_named_resources_absent(admin_url: &str, name: &str) -> Result<()> {
    validate_safe_label(name, "resource name")?;
    let mut admin = PgConnection::connect(admin_url).await?;
    let database_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pg_database WHERE datname = $1")
            .bind(name)
            .fetch_one(&mut admin)
            .await?;
    let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_roles WHERE rolname = $1")
        .bind(name)
        .fetch_one(&mut admin)
        .await?;
    admin.close().await?;

    if database_count != 0 || role_count != 0 {
        bail!(
            "ephemeral migration resource remained: databases={database_count}, roles={role_count}"
        );
    }
    Ok(())
}

async fn drop_database(admin_url: &str, database_name: &str) -> Result<()> {
    validate_safe_label(database_name, "database name")?;
    let mut admin = PgConnection::connect(admin_url).await?;
    admin
        .execute(format!("DROP DATABASE IF EXISTS {database_name} WITH (FORCE)").as_str())
        .await
        .context("drop the ephemeral PostgreSQL migration-test database")?;
    admin.close().await?;
    Ok(())
}

async fn verify_admin_connection(admin_url: &str) -> Result<()> {
    let mut admin = PgConnection::connect(admin_url).await?;
    let can_manage_test_resources: bool = sqlx::query_scalar(
        "SELECT rolsuper OR (rolcreatedb AND rolcreaterole) \
         FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&mut admin)
    .await?;
    admin.close().await?;

    if !can_manage_test_resources {
        bail!("PostgreSQL migration-test user must have CREATEDB and CREATEROLE");
    }
    Ok(())
}

fn validated_admin_url(value: &str) -> Result<Url> {
    validate_admin_url(
        value,
        std::env::var("NVBES_ENV").ok().as_deref(),
        std::env::var("NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE")
            .ok()
            .as_deref(),
    )
}

fn validate_admin_url(
    value: &str,
    environment: Option<&str>,
    destructive_opt_in: Option<&str>,
) -> Result<Url> {
    crate::test_support::validate_test_database_url(value, environment, destructive_opt_in)
        .map_err(anyhow::Error::msg)?;
    let parsed = Url::parse(value).context("parse PostgreSQL migration-test admin URL")?;
    Ok(parsed)
}

fn migration_run_id() -> Result<&'static str> {
    static RUN_ID: OnceLock<String> = OnceLock::new();
    let run_id = RUN_ID.get_or_init(|| {
        std::env::var("IDENTITY_MIGRATION_TEST_RUN_ID")
            .unwrap_or_else(|_| Uuid::new_v4().simple().to_string()[..12].to_owned())
    });
    validate_safe_label(run_id, "migration run id")?;
    if run_id.len() > 16 {
        bail!("migration run id must contain at most 16 characters");
    }
    Ok(run_id)
}

fn validate_safe_label(value: &str, description: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        bail!("{description} must contain only lowercase ASCII letters, digits, and underscores");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_admin_url, validate_safe_label};

    #[test]
    fn admin_url_guard_accepts_only_explicit_loopback_test_databases() {
        for url in [
            "postgres://user:password@localhost:5432/source_test",
            "postgres://user:password@127.0.0.1:5432/test_source",
            "postgresql://user:password@[::1]:5432/source_integration_test",
        ] {
            assert!(
                validate_admin_url(url, Some("test"), Some("account-quality-v1")).is_ok(),
                "{url} should be accepted"
            );
        }

        for url in [
            "postgres://user:password@localhost.evil.test:5432/source_test",
            "postgres://user:password@10.0.0.5:5432/source_test",
            "postgres://user:password@192.168.1.5:5432/source_test",
            "https://localhost:5432/source_test",
            "postgres://user:password@localhost:5432/",
            "postgres://user:password@localhost:5432/source",
            "postgres://user:password@localhost:5432/source_production_test",
        ] {
            assert!(
                validate_admin_url(url, Some("test"), Some("account-quality-v1")).is_err(),
                "{url} should be rejected"
            );
        }
        assert!(
            validate_admin_url(
                "postgres://user:password@localhost:5432/source_test",
                Some("test"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn generated_identifier_guard_rejects_sql_metacharacters() {
        assert!(validate_safe_label("safe_test_123", "label").is_ok());
        for label in ["", "UPPER", "semi;colon", "quoted_name\"", "dash-name"] {
            assert!(validate_safe_label(label, "label").is_err());
        }
    }
}
