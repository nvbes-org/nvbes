use reqwest::Url;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

pub(super) struct GrpcTestDatabase {
    admin: PgPool,
    pub owner: PgPool,
    pub runtime: PgPool,
    database_name: String,
}

impl GrpcTestDatabase {
    pub async fn create() -> anyhow::Result<Self> {
        let source_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
        require_local_disposable_cluster(&source_url)?;
        let database_name = format!("cloud_grpc_{}", Uuid::new_v4().simple());
        let admin_url = replace_database(&source_url, "postgres")?;
        let database_url = replace_database(&source_url, &database_name)?;
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&admin_url)
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "Cloud gRPC RLS tests require a local PostgreSQL maintenance connection: \
                     {error}"
                )
            })?;
        sqlx::query(&format!("CREATE DATABASE {database_name}"))
            .execute(&admin)
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "Cloud gRPC RLS tests require CREATEDB on the local PostgreSQL role: {error}"
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
                return Err(error.into());
            }
        };
        if let Err(error) = sqlx::migrate!("./migrations").run(&owner).await {
            owner.close().await;
            drop_database(&admin, &database_name).await.ok();
            return Err(error.into());
        }
        let runtime = match nvbes_core::postgres_runtime::connect_pool_with_url_and_role(
            nvbes_core::postgres_runtime::PostgresPoolSettings::from_max_connections(1),
            &database_url,
            "nvbes_app",
        )
        .await
        {
            Ok(runtime) => runtime,
            Err(error) => {
                owner.close().await;
                drop_database(&admin, &database_name).await.ok();
                return Err(error.into());
            }
        };

        Ok(Self {
            admin,
            owner,
            runtime,
            database_name,
        })
    }

    pub async fn cleanup(self) -> anyhow::Result<()> {
        self.runtime.close().await;
        self.owner.close().await;
        let result = drop_database(&self.admin, &self.database_name).await;
        self.admin.close().await;
        result
    }
}

fn require_local_disposable_cluster(database_url: &str) -> anyhow::Result<()> {
    let url = Url::parse(database_url)?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL test URL must contain a host"))?;
    anyhow::ensure!(
        matches!(host, "localhost" | "127.0.0.1" | "::1"),
        "destructive Cloud gRPC tests only accept a loopback PostgreSQL cluster; \
         refusing host {host:?}"
    );
    anyhow::ensure!(
        !matches!(
            url.path().trim_start_matches('/'),
            "" | "postgres" | "template0" | "template1"
        ),
        "DATABASE_URL must name a disposable application database"
    );
    Ok(())
}

fn replace_database(database_url: &str, database_name: &str) -> anyhow::Result<String> {
    let mut url = Url::parse(database_url)?;
    url.set_path(&format!("/{database_name}"));
    Ok(url.to_string())
}

async fn drop_database(admin: &PgPool, database_name: &str) -> anyhow::Result<()> {
    sqlx::query(
        "SELECT pg_terminate_backend(pid)
         FROM pg_stat_activity
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
