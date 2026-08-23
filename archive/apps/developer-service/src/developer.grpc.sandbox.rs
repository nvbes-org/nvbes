use chrono::{DateTime, Utc};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

#[path = "developer.grpc.sandbox.fixtures.rs"]
mod fixtures;

const DEFAULT_SANDBOX_DATA_PROFILE: &str = "minimal";
const SANDBOX_TENANT_KIND: &str = "team";
const SANDBOX_SECURITY_TIER: &str = "sandbox";

pub async fn get_sandbox(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<developer::GetSandboxResponse, Status> {
    Ok(developer::GetSandboxResponse {
        sandbox: find_sandbox(db, tenant_id).await?,
    })
}

pub async fn create_sandbox(
    db: &sqlx::PgPool,
    request: developer::CreateSandboxRequest,
) -> Result<developer::Sandbox, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let data_profile = normalize_data_profile(&request.template)?;

    if let Some(existing) = find_sandbox(db, tenant_id).await? {
        return update_sandbox_profile(db, tenant_id, &existing.sandbox_id, data_profile).await;
    }

    let sandbox_tenant_id = Uuid::new_v4();
    let slug = format!(
        "sandbox-{}-{}",
        tenant_id.simple(),
        sandbox_tenant_id.simple()
    );
    let name = "Developer Sandbox".to_string();

    let mut tx = db.begin().await.map_err(sql_status)?;
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, $2::tenant_kind, $3, $4, 'active', $5)
        "#,
    )
    .bind(sandbox_tenant_id)
    .bind(SANDBOX_TENANT_KIND)
    .bind(&name)
    .bind(&slug)
    .bind(SANDBOX_SECURITY_TIER)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;

    let sandbox = sqlx::query(
        r#"
        INSERT INTO developer_sandbox_tenants (
          tenant_id,
          sandbox_tenant_id,
          status,
          data_profile
        )
        VALUES ($1, $2, 'active', $3)
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          $4::text AS sandbox_name,
          $5::text AS sandbox_slug,
          status::text AS status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(sandbox_tenant_id)
    .bind(data_profile)
    .bind(&name)
    .bind(&slug)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;

    Ok(sandbox_from_row(sandbox))
}

pub async fn reset_sandbox(
    db: &sqlx::PgPool,
    request: developer::ResetSandboxRequest,
) -> Result<developer::Sandbox, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let sandbox = mark_resetting(db, tenant_id).await?;
    let sandbox_tenant_id = parse_uuid(&sandbox.sandbox_id, "sandbox_id")?;
    fixtures::provision_sandbox(db, sandbox_tenant_id, &sandbox.data_profile).await?;
    mark_active(db, tenant_id).await
}

async fn find_sandbox(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Option<developer::Sandbox>, Status> {
    sqlx::query(&sandbox_select_sql("WHERE s.tenant_id = $1"))
        .bind(tenant_id)
        .fetch_optional(db)
        .await
        .map_err(sql_status)
        .map(|row| row.map(sandbox_from_row))
}

async fn update_sandbox_profile(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    sandbox_id: &str,
    data_profile: &str,
) -> Result<developer::Sandbox, Status> {
    sqlx::query(
        r#"
        UPDATE developer_sandbox_tenants
        SET data_profile = $2,
            status = 'active',
            updated_at = now()
        WHERE tenant_id = $1
          AND sandbox_tenant_id = $3
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
          (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
          status::text AS status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(data_profile)
    .bind(parse_uuid(sandbox_id, "sandbox_id")?)
    .fetch_one(db)
    .await
    .map_err(sql_status)
    .map(sandbox_from_row)
}

async fn mark_resetting(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<developer::Sandbox, Status> {
    sqlx::query(
        r#"
        UPDATE developer_sandbox_tenants
        SET status = 'resetting',
            reset_requested_at = now(),
            updated_at = now()
        WHERE tenant_id = $1
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
          (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
          status::text AS status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .map(sandbox_from_row)
    .ok_or_else(|| Status::not_found("developer sandbox was not found"))
}

async fn mark_active(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<developer::Sandbox, Status> {
    sqlx::query(
        r#"
        UPDATE developer_sandbox_tenants
        SET status = 'active',
            updated_at = now()
        WHERE tenant_id = $1
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
          (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
          status::text AS status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)
    .map(sandbox_from_row)
}

pub(crate) async fn purge_developer_sandbox_data(
    tx: &mut Transaction<'_, Postgres>,
    sandbox_tenant_id: Uuid,
) -> Result<(), Status> {
    sqlx::query("DELETE FROM developer_health_checks WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    sqlx::query("DELETE FROM developer_webhook_endpoints WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    Ok(())
}

fn sandbox_select_sql(predicate: &str) -> String {
    format!(
        r#"
        SELECT
          s.tenant_id,
          s.sandbox_tenant_id,
          t.name AS sandbox_name,
          t.slug AS sandbox_slug,
          s.status::text AS status,
          s.data_profile,
          s.reset_requested_at,
          s.updated_at
        FROM developer_sandbox_tenants s
        INNER JOIN tenants t ON t.id = s.sandbox_tenant_id
        {predicate}
        "#
    )
}

fn sandbox_from_row(row: PgRow) -> developer::Sandbox {
    developer::Sandbox {
        sandbox_id: row.get::<Uuid, _>("sandbox_tenant_id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        status: row.get("status"),
        reset_at: row
            .get::<Option<DateTime<Utc>>, _>("reset_requested_at")
            .map(time_string)
            .unwrap_or_default(),
        sandbox_name: row.get("sandbox_name"),
        sandbox_slug: row.get("sandbox_slug"),
        data_profile: row.get("data_profile"),
        updated_at: time_string(row.get("updated_at")),
    }
}

fn normalize_data_profile(data_profile: &str) -> Result<&'static str, Status> {
    match data_profile.trim() {
        "" => Ok(DEFAULT_SANDBOX_DATA_PROFILE),
        "minimal" => Ok("minimal"),
        "oauth" => Ok("oauth"),
        "full" => Ok("full"),
        _ => Err(Status::invalid_argument(
            "Sandbox data profile must be minimal, oauth, or full.",
        )),
    }
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

#[cfg(test)]
#[path = "developer.grpc.sandbox.contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
    use super::normalize_data_profile;

    #[test]
    fn sandbox_profile_defaults_to_minimal() {
        assert_eq!(normalize_data_profile("").unwrap(), "minimal");
        assert_eq!(normalize_data_profile(" ").unwrap(), "minimal");
    }

    #[test]
    fn sandbox_profile_accepts_known_profiles() {
        assert_eq!(normalize_data_profile("oauth").unwrap(), "oauth");
        assert_eq!(normalize_data_profile("full").unwrap(), "full");
    }

    #[test]
    fn sandbox_profile_rejects_unknown_profile() {
        assert!(normalize_data_profile("production").is_err());
    }
}
