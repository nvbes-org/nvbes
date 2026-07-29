use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use url::Url;
use uuid::Uuid;

const TEST_ROLE_PASSWORD: &str = "AccountRlsTestOnly42";

#[derive(Clone, Copy)]
pub enum RlsSchema {
    ProductionBaseline,
    HardenedReference,
}

pub struct RlsTestDatabase {
    admin: PgPool,
    pub owner: PgPool,
    pub runtime: PgPool,
    database_url: String,
    database_name: String,
    runtime_role: String,
}

impl RlsTestDatabase {
    pub async fn create(schema: RlsSchema) -> anyhow::Result<Self> {
        let source_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/nvbes_test".to_string()
            });
        require_local_disposable_cluster(&source_url)?;
        let suffix = Uuid::new_v4().simple().to_string();
        let database_name = format!("account_rls_{suffix}");
        let runtime_role = format!("account_rls_runtime_{suffix}");
        let admin_url = replace_database(&source_url, "postgres")?;
        let database_url = replace_database(&source_url, &database_name)?;
        let runtime_url =
            replace_credentials(&database_url, &runtime_role, Some(TEST_ROLE_PASSWORD))?;

        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&admin_url)
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "RLS integration tests require a PostgreSQL migrator that can connect \
                     to the postgres maintenance database: {error}"
                )
            })?;
        sqlx::query(&format!("CREATE DATABASE {database_name}"))
            .execute(&admin)
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "RLS integration tests require CREATEDB on the PostgreSQL migrator: {error}"
                )
            })?;

        let owner = match PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
        {
            Ok(owner) => owner,
            Err(error) => {
                drop_database(&admin, &database_name).await.ok();
                admin.close().await;
                return Err(error.into());
            }
        };
        if let Err(error) = crate::database::run_migrations(&owner).await {
            cleanup_failed_setup(&admin, owner, &database_name, &runtime_role).await;
            return Err(error);
        }
        if matches!(schema, RlsSchema::HardenedReference)
            && let Err(error) = sqlx::raw_sql(include_str!("identity.database.rls.reference.sql"))
                .execute(&owner)
                .await
        {
            cleanup_failed_setup(&admin, owner, &database_name, &runtime_role).await;
            return Err(error.into());
        }

        if let Err(error) = provision_runtime_role(&owner, &runtime_role, TEST_ROLE_PASSWORD).await
        {
            cleanup_failed_setup(&admin, owner, &database_name, &runtime_role).await;
            return Err(error);
        }
        let runtime = match PgPoolOptions::new()
            .max_connections(1)
            .connect(&runtime_url)
            .await
        {
            Ok(runtime) => runtime,
            Err(error) => {
                cleanup_failed_setup(&admin, owner, &database_name, &runtime_role).await;
                return Err(error.into());
            }
        };

        Ok(Self {
            admin,
            owner,
            runtime,
            database_url,
            database_name,
            runtime_role,
        })
    }

    pub async fn connect_role_scoped_runtime(&self) -> anyhow::Result<PgPool> {
        Ok(
            nvbes_core::postgres_runtime::connect_pool_with_url_and_role(
                nvbes_core::postgres_runtime::PostgresPoolSettings::from_max_connections(1),
                &self.database_url,
                &self.runtime_role,
            )
            .await?,
        )
    }

    pub async fn cleanup(self) -> anyhow::Result<()> {
        self.runtime.close().await;
        self.owner.close().await;
        drop_database(&self.admin, &self.database_name).await?;
        sqlx::query(&format!("DROP ROLE IF EXISTS {}", self.runtime_role))
            .execute(&self.admin)
            .await?;
        self.admin.close().await;
        Ok(())
    }
}

fn require_local_disposable_cluster(database_url: &str) -> anyhow::Result<()> {
    crate::test_support::validate_test_database_url(
        database_url,
        std::env::var("NVBES_ENV").ok().as_deref(),
        std::env::var("NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE")
            .ok()
            .as_deref(),
    )
    .map_err(anyhow::Error::msg)
}

async fn provision_runtime_role(
    owner: &PgPool,
    runtime_role: &str,
    password: &str,
) -> anyhow::Result<()> {
    sqlx::query(&format!(
        "CREATE ROLE {runtime_role} LOGIN PASSWORD '{password}' \
         NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS"
    ))
    .execute(owner)
    .await
    .map_err(|error| {
        anyhow::anyhow!(
            "RLS integration tests require CREATEROLE on the PostgreSQL migrator: {error}"
        )
    })?;
    let database_name = current_database_name(owner).await?;
    for statement in [
        format!("GRANT CONNECT ON DATABASE {database_name} TO {runtime_role}"),
        format!("GRANT USAGE ON SCHEMA public TO {runtime_role}"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {runtime_role}"
        ),
        format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {runtime_role}"),
        format!("REVOKE CREATE ON SCHEMA public FROM {runtime_role}"),
    ] {
        sqlx::query(&statement).execute(owner).await?;
    }
    Ok(())
}

async fn current_database_name(pool: &PgPool) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(pool)
        .await?)
}

async fn drop_database(admin: &PgPool, database_name: &str) -> anyhow::Result<()> {
    sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
         WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(database_name)
    .execute(admin)
    .await?;
    sqlx::query(&format!("DROP DATABASE IF EXISTS {database_name}"))
        .execute(admin)
        .await?;
    Ok(())
}

async fn cleanup_failed_setup(
    admin: &PgPool,
    owner: PgPool,
    database_name: &str,
    runtime_role: &str,
) {
    owner.close().await;
    drop_database(admin, database_name).await.ok();
    sqlx::query(&format!("DROP ROLE IF EXISTS {runtime_role}"))
        .execute(admin)
        .await
        .ok();
    admin.close().await;
}

fn replace_database(database_url: &str, database_name: &str) -> anyhow::Result<String> {
    let mut url = Url::parse(database_url)?;
    url.set_path(&format!("/{database_name}"));
    Ok(url.to_string())
}

fn replace_credentials(
    database_url: &str,
    username: &str,
    password: Option<&str>,
) -> anyhow::Result<String> {
    let mut url = Url::parse(database_url)?;
    url.set_username(username)
        .map_err(|_| anyhow::anyhow!("generated PostgreSQL username must be valid"))?;
    url.set_password(password)
        .map_err(|_| anyhow::anyhow!("generated PostgreSQL password must be valid"))?;
    Ok(url.to_string())
}
