use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::developer::types::DeveloperSandboxTenantSummary, http::error::AppError};

#[path = "identity.domains.developer.sandbox.fixtures.rs"]
mod fixtures;

const DEFAULT_SANDBOX_DATA_PROFILE: &str = "minimal";
const SANDBOX_TENANT_KIND: &str = "team";
const SANDBOX_SECURITY_TIER: &str = "sandbox";

pub async fn find_sandbox(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Option<DeveloperSandboxTenantSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          s.tenant_id,
          s.sandbox_tenant_id,
          t.name AS sandbox_name,
          t.slug AS sandbox_slug,
          s.status,
          s.data_profile,
          s.reset_requested_at,
          s.updated_at
        FROM developer_sandbox_tenants s
        INNER JOIN tenants t ON t.id = s.sandbox_tenant_id
        WHERE s.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

pub async fn upsert_sandbox(
    db: &PgPool,
    tenant_id: Uuid,
    requested_data_profile: Option<&str>,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    let data_profile = normalize_data_profile(requested_data_profile)?;

    if let Some(existing) = find_sandbox(db, tenant_id).await? {
        return update_sandbox_profile(db, existing.tenant_id, data_profile).await;
    }

    let sandbox_tenant_id = Uuid::new_v4();
    let slug = format!(
        "sandbox-{}-{}",
        tenant_id.simple(),
        sandbox_tenant_id.simple()
    );
    let name = "Developer Sandbox".to_string();

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
    .execute(db)
    .await?;

    sqlx::query_as(
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
          status,
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
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn reset_sandbox(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    let sandbox = mark_resetting(db, tenant_id).await?;
    fixtures::provision_sandbox(db, sandbox.sandbox_tenant_id, &sandbox.data_profile).await?;
    mark_active(db, tenant_id).await
}

fn normalize_data_profile(data_profile: Option<&str>) -> Result<&'static str, AppError> {
    match data_profile
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SANDBOX_DATA_PROFILE)
    {
        "minimal" => Ok("minimal"),
        "oauth" => Ok("oauth"),
        "full" => Ok("full"),
        _ => Err(AppError::bad_request(
            "invalid_sandbox_data_profile",
            "Sandbox data profile must be minimal, oauth, or full.",
        )),
    }
}

async fn update_sandbox_profile(
    db: &PgPool,
    tenant_id: Uuid,
    data_profile: &str,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    sqlx::query_as(
        r#"
        UPDATE developer_sandbox_tenants
        SET data_profile = $2,
            status = 'active',
            updated_at = now()
        WHERE tenant_id = $1
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
          (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
          status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(data_profile)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

async fn mark_resetting(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    sqlx::query_as(
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
          status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("sandbox_not_found", "Sandbox tenant not found."))
}

async fn mark_active(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    sqlx::query_as(
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
          status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

#[cfg(test)]
#[path = "identity.domains.developer.sandbox.tests.rs"]
mod tests;
