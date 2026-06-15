use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub struct TrustTenantRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub security_tier: String,
}

#[derive(Debug, FromRow)]
pub struct TrustMfaRow {
    pub active_members: i64,
    pub members_with_mfa: i64,
    pub active_factors: i64,
    pub passkey_factors: i64,
}

#[derive(Debug, FromRow)]
pub struct TrustSsoProviderRow {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    pub provider_family: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct TrustDomainRow {
    pub id: Uuid,
    pub domain: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub sso_required: bool,
}

#[derive(Debug, FromRow)]
pub struct TrustHostingRegionRow {
    pub data_region: String,
    pub legal_jurisdiction: String,
    pub workspace_count: i64,
}

pub async fn tenant(db: &PgPool, tenant_id: Uuid) -> Result<TrustTenantRow, AppError> {
    Ok(sqlx::query_as::<_, TrustTenantRow>(
        r#"
        SELECT id, name, slug, status::text AS status, security_tier
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?)
}

pub async fn mfa(db: &PgPool, tenant_id: Uuid) -> Result<TrustMfaRow, AppError> {
    Ok(sqlx::query_as::<_, TrustMfaRow>(
        r#"
        SELECT
          COUNT(DISTINCT tm.principal_id)::bigint AS active_members,
          COUNT(DISTINCT tm.principal_id) FILTER (WHERE mf.id IS NOT NULL)::bigint AS members_with_mfa,
          COUNT(DISTINCT mf.id)::bigint AS active_factors,
          COUNT(DISTINCT mf.id) FILTER (WHERE mf.factor_type = 'webauthn')::bigint AS passkey_factors
        FROM tenant_memberships tm
        LEFT JOIN mfa_factors mf ON mf.principal_id = tm.principal_id AND mf.status = 'active'
        WHERE tm.tenant_id = $1 AND tm.status = 'active'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?)
}

pub async fn sso_providers(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TrustSsoProviderRow>, AppError> {
    Ok(sqlx::query_as::<_, TrustSsoProviderRow>(
        r#"
        SELECT id, name, provider_type::text AS provider_type,
          provider_family, status, created_at
        FROM federated_identity_providers
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn domains(db: &PgPool, tenant_id: Uuid) -> Result<Vec<TrustDomainRow>, AppError> {
    Ok(sqlx::query_as::<_, TrustDomainRow>(
        r#"
        SELECT id, domain, verified_at, sso_required
        FROM tenant_domains
        WHERE tenant_id = $1
        ORDER BY verified_at DESC NULLS LAST, domain ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn hosting_regions(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TrustHostingRegionRow>, AppError> {
    Ok(sqlx::query_as::<_, TrustHostingRegionRow>(
        r#"
        SELECT data_region::text AS data_region,
          jurisdiction::text AS legal_jurisdiction,
          COUNT(*)::bigint AS workspace_count
        FROM workspaces
        WHERE tenant_id = $1
        GROUP BY data_region, jurisdiction
        ORDER BY workspace_count DESC, data_region ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}
