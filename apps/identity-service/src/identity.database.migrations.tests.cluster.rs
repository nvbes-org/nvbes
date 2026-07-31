use std::collections::BTreeSet;
use std::thread;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, Executor, PgConnection};

const ACCOUNT_MIGRATION_LOCK: i64 = 0x4e56_4245_534d_4947;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleSnapshot {
    schema_version: u8,
    system_role_preexisted: bool,
    roles_before: BTreeSet<String>,
}

pub struct ClusterRoleGuard {
    admin: Option<PgConnection>,
    database_prefix: String,
    marker_role: String,
    snapshot: RoleSnapshot,
    active: bool,
}

impl ClusterRoleGuard {
    pub async fn acquire(admin_url: &str, run_id: &str, database_prefix: &str) -> Result<Self> {
        validate_identifier(run_id)?;
        validate_identifier_prefix(database_prefix)?;
        let marker_role = format!("nvbes_acct_mig_marker_{run_id}");
        validate_identifier(&marker_role)?;

        let mut admin = PgConnection::connect(admin_url).await?;
        acquire_cluster_lock(&mut admin).await?;
        let roles_before = role_names(&mut admin).await?;
        if roles_before.contains(&marker_role) {
            bail!("migration-test cluster marker already exists for this run");
        }
        let snapshot = RoleSnapshot {
            schema_version: 1,
            system_role_preexisted: roles_before.contains("nvbes_system"),
            roles_before,
        };
        create_snapshot_marker(&mut admin, &marker_role, &snapshot).await?;

        Ok(Self {
            admin: Some(admin),
            database_prefix: database_prefix.to_owned(),
            marker_role,
            snapshot,
            active: true,
        })
    }

    pub async fn cleanup(mut self) -> Result<()> {
        let admin = self
            .admin
            .as_mut()
            .context("cluster-role guard lost its PostgreSQL connection")?;
        cleanup_role_state(
            admin,
            &self.database_prefix,
            &self.marker_role,
            &self.snapshot,
        )
        .await?;
        release_cluster_lock(admin).await?;
        self.active = false;
        if let Some(admin) = self.admin.take() {
            admin.close().await?;
        }
        Ok(())
    }
}

impl Drop for ClusterRoleGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let Some(mut admin) = self.admin.take() else {
            return;
        };
        let database_prefix = self.database_prefix.clone();
        let marker_role = self.marker_role.clone();
        let snapshot = self.snapshot.clone();

        let cleanup = thread::Builder::new()
            .name("account-migration-role-cleanup".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(async move {
                    cleanup_role_state(&mut admin, &database_prefix, &marker_role, &snapshot)
                        .await?;
                    release_cluster_lock(&mut admin).await?;
                    admin.close().await?;
                    Result::<()>::Ok(())
                })
            });

        match cleanup {
            Ok(handle) => match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("failed to clean migration-test cluster roles: {error:#}");
                }
                Err(_) => eprintln!("migration-test cluster-role cleanup thread panicked"),
            },
            Err(error) => eprintln!("failed to start cluster-role cleanup: {error}"),
        }
    }
}

pub async fn recover_interrupted_run(
    admin_url: &str,
    run_id: &str,
    database_prefix: &str,
) -> Result<()> {
    validate_identifier(run_id)?;
    validate_identifier_prefix(database_prefix)?;
    let marker_role = format!("nvbes_acct_mig_marker_{run_id}");
    validate_identifier(&marker_role)?;

    let mut admin = PgConnection::connect(admin_url).await?;
    acquire_cluster_lock(&mut admin).await?;
    let snapshot = read_snapshot_marker(&mut admin, &marker_role).await?;

    drop_run_databases(&mut admin, database_prefix).await?;
    if let Some(snapshot) = snapshot {
        cleanup_role_state(&mut admin, database_prefix, &marker_role, &snapshot).await?;
    } else {
        let (databases, markers): (i64, i64) = sqlx::query_as(
            "SELECT \
             (SELECT COUNT(*) FROM pg_database WHERE starts_with(datname, $1)), \
             (SELECT COUNT(*) FROM pg_roles WHERE rolname = $2)",
        )
        .bind(database_prefix)
        .bind(&marker_role)
        .fetch_one(&mut admin)
        .await?;
        if databases != 0 || markers != 0 {
            bail!("migration cleanup left a database or cluster-role marker");
        }
    }
    release_cluster_lock(&mut admin).await?;
    admin.close().await?;
    Ok(())
}

async fn create_snapshot_marker(
    admin: &mut PgConnection,
    marker_role: &str,
    snapshot: &RoleSnapshot,
) -> Result<()> {
    let payload = serde_json::to_string(snapshot)?;
    let comment_statement: String =
        sqlx::query_scalar("SELECT format('COMMENT ON ROLE %I IS %L', $1, $2)")
            .bind(marker_role)
            .bind(payload)
            .fetch_one(&mut *admin)
            .await?;

    admin.execute("BEGIN").await?;
    let result = async {
        admin
            .execute(format!("CREATE ROLE {marker_role} NOLOGIN").as_str())
            .await?;
        admin.execute(comment_statement.as_str()).await?;
        Result::<()>::Ok(())
    }
    .await;
    match result {
        Ok(()) => {
            admin.execute("COMMIT").await?;
            Ok(())
        }
        Err(error) => {
            let _ = admin.execute("ROLLBACK").await;
            Err(error)
        }
    }
}

async fn read_snapshot_marker(
    admin: &mut PgConnection,
    marker_role: &str,
) -> Result<Option<RoleSnapshot>> {
    let payload: Option<String> = sqlx::query_scalar(
        "SELECT shobj_description(oid, 'pg_authid') FROM pg_roles WHERE rolname = $1",
    )
    .bind(marker_role)
    .fetch_optional(admin)
    .await?
    .flatten();
    let snapshot: Option<RoleSnapshot> = payload
        .map(|payload| serde_json::from_str(&payload).context("decode cluster-role snapshot"))
        .transpose()?;
    if snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.schema_version != 1)
    {
        bail!("unsupported cluster-role snapshot version");
    }
    Ok(snapshot)
}

async fn cleanup_role_state(
    admin: &mut PgConnection,
    database_prefix: &str,
    marker_role: &str,
    snapshot: &RoleSnapshot,
) -> Result<()> {
    let database_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pg_database WHERE starts_with(datname, $1)")
            .bind(database_prefix)
            .fetch_one(&mut *admin)
            .await?;
    if database_count != 0 {
        bail!("refusing cluster-role cleanup while migration-test databases remain");
    }

    if !snapshot.system_role_preexisted {
        admin.execute("DROP ROLE IF EXISTS nvbes_system").await?;
    }
    let mut expected_with_marker = snapshot.roles_before.clone();
    expected_with_marker.insert(marker_role.to_owned());
    if role_names(admin).await? != expected_with_marker {
        bail!("migration introduced an unexpected cluster-wide role");
    }
    admin
        .execute(format!("DROP ROLE IF EXISTS {marker_role}").as_str())
        .await?;

    if role_names(admin).await? != snapshot.roles_before {
        bail!("cluster roles differ from the pre-migration snapshot");
    }
    Ok(())
}

async fn drop_run_databases(admin: &mut PgConnection, database_prefix: &str) -> Result<()> {
    let databases: Vec<String> =
        sqlx::query_scalar("SELECT datname FROM pg_database WHERE starts_with(datname, $1)")
            .bind(database_prefix)
            .fetch_all(&mut *admin)
            .await?;
    for database in databases {
        validate_identifier(&database)?;
        admin
            .execute(format!("DROP DATABASE IF EXISTS {database} WITH (FORCE)").as_str())
            .await?;
    }
    Ok(())
}

async fn role_names(admin: &mut PgConnection) -> Result<BTreeSet<String>> {
    Ok(
        sqlx::query_scalar("SELECT rolname FROM pg_roles ORDER BY rolname")
            .fetch_all(admin)
            .await?
            .into_iter()
            .collect(),
    )
}

async fn acquire_cluster_lock(admin: &mut PgConnection) -> Result<()> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(ACCOUNT_MIGRATION_LOCK)
        .execute(admin)
        .await?;
    Ok(())
}

async fn release_cluster_lock(admin: &mut PgConnection) -> Result<()> {
    let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(ACCOUNT_MIGRATION_LOCK)
        .fetch_one(admin)
        .await?;
    if !released {
        bail!("migration-test cluster advisory lock was not held");
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 63
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        bail!("unsafe PostgreSQL migration-test identifier");
    }
    Ok(())
}

fn validate_identifier_prefix(value: &str) -> Result<()> {
    validate_identifier(value.trim_end_matches('_'))
}
