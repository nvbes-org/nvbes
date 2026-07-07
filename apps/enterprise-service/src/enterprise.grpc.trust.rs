use chrono::{DateTime, Utc};
use sqlx::Row;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{pb::nvbes::enterprise::v1 as enterprise, service_status::sql_status};

pub async fn trust_center(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::TrustCenter, Status> {
    let tenant = tenant(db, tenant_id).await?;
    let mfa = mfa(db, tenant_id).await?;
    let providers = sso_providers(db, tenant_id).await?;
    let domains = domains(db, tenant_id).await?;
    let hosting_regions = hosting_regions(db, tenant_id).await?;
    let audit_events = audit_events(db, tenant_id).await?;

    let active_providers = providers
        .iter()
        .filter(|provider| provider.status == "active")
        .count() as i64;
    let required_domains = domains.iter().filter(|domain| domain.sso_required).count() as i64;

    Ok(enterprise::TrustCenter {
        tenant: Some(tenant),
        mfa: Some(enterprise::TrustCenterMfaStatus {
            enabled: mfa.members_with_mfa > 0,
            active_members: mfa.active_members,
            members_with_mfa: mfa.members_with_mfa,
            active_factors: mfa.active_factors,
            passkey_factors: mfa.passkey_factors,
        }),
        sso: Some(enterprise::TrustCenterSsoStatus {
            enabled: active_providers > 0 || required_domains > 0,
            active_providers,
            required_domains,
            providers: providers.into_iter().map(provider_from_row).collect(),
        }),
        verified_domains: domains.into_iter().map(domain_from_row).collect(),
        audit: Some(enterprise::TrustCenterAuditStatus {
            immutable: true,
            recent_events: audit_events.into_iter().map(audit_event_from_row).collect(),
        }),
        hosting_regions: hosting_regions
            .into_iter()
            .map(hosting_region_from_row)
            .collect(),
        dpa: Some(enterprise::TrustCenterDocument {
            name: "Data Processing Agreement".to_string(),
            status: "ready_for_signature".to_string(),
            version: "2026-05-11".to_string(),
            url: "/legal/data-processing-agreement".to_string(),
        }),
        subprocessors: Vec::new(),
        generated_at: Utc::now().to_rfc3339(),
    })
}

async fn tenant(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::TrustCenterTenant, Status> {
    let row = sqlx::query(
        r#"
        SELECT id, name, slug, status::text AS status, security_tier
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::TrustCenterTenant {
        tenant_id: row.get::<Uuid, _>("id").to_string(),
        name: row.get("name"),
        slug: row.get("slug"),
        status: row.get("status"),
        security_tier: row.get("security_tier"),
    })
}

async fn mfa(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<TrustMfaRow, Status> {
    sqlx::query_as::<_, TrustMfaRow>(
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
    .await
    .map_err(sql_status)
}

async fn sso_providers(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TrustSsoProviderRow>, Status> {
    sqlx::query_as::<_, TrustSsoProviderRow>(
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
    .await
    .map_err(sql_status)
}

async fn domains(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<Vec<TrustDomainRow>, Status> {
    sqlx::query_as::<_, TrustDomainRow>(
        r#"
        SELECT id, domain, verified_at, sso_required, sso_provider_id
        FROM tenant_domains
        WHERE tenant_id = $1
        ORDER BY verified_at DESC NULLS LAST, domain ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
}

async fn hosting_regions(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TrustHostingRegionRow>, Status> {
    sqlx::query_as::<_, TrustHostingRegionRow>(
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
    .await
    .map_err(sql_status)
}

async fn audit_events(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TrustAuditEventRow>, Status> {
    sqlx::query_as::<_, TrustAuditEventRow>(
        r#"
        SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
          u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        WHERE ae.tenant_id = $1
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT 10
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
}

fn provider_from_row(row: TrustSsoProviderRow) -> enterprise::TrustCenterSsoProvider {
    enterprise::TrustCenterSsoProvider {
        provider_id: row.id.to_string(),
        name: row.name,
        provider_type: row.provider_type,
        provider_family: row.provider_family,
        status: row.status,
        created_at: row.created_at.to_rfc3339(),
    }
}

fn domain_from_row(row: TrustDomainRow) -> enterprise::TrustCenterDomain {
    enterprise::TrustCenterDomain {
        domain_id: row.id.to_string(),
        domain: row.domain,
        verified: row.verified_at.is_some(),
        sso_required: row.sso_required,
        sso_provider_id: optional_uuid_string(row.sso_provider_id),
        verified_at: optional_time_string(row.verified_at),
    }
}

fn hosting_region_from_row(row: TrustHostingRegionRow) -> enterprise::TrustCenterHostingRegion {
    enterprise::TrustCenterHostingRegion {
        data_region: row.data_region,
        legal_jurisdiction: row.legal_jurisdiction,
        workspace_count: row.workspace_count,
    }
}

fn audit_event_from_row(row: TrustAuditEventRow) -> enterprise::TrustCenterAuditEvent {
    enterprise::TrustCenterAuditEvent {
        event_id: row.id.to_string(),
        event_type: row.event_type,
        actor_id: optional_uuid_string(row.actor_id),
        actor_email: row.actor_email.unwrap_or_default(),
        target_type: row.target_type.unwrap_or_default(),
        target_id: optional_uuid_string(row.target_id),
        metadata_json: row.metadata.to_string(),
        created_at: row.created_at.to_rfc3339(),
    }
}

fn optional_time_string(value: Option<DateTime<Utc>>) -> String {
    value.map(|time| time.to_rfc3339()).unwrap_or_default()
}

fn optional_uuid_string(value: Option<Uuid>) -> String {
    value.map(|id| id.to_string()).unwrap_or_default()
}

#[derive(sqlx::FromRow)]
struct TrustMfaRow {
    active_members: i64,
    members_with_mfa: i64,
    active_factors: i64,
    passkey_factors: i64,
}

#[derive(sqlx::FromRow)]
struct TrustSsoProviderRow {
    id: Uuid,
    name: String,
    provider_type: String,
    provider_family: String,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TrustDomainRow {
    id: Uuid,
    domain: String,
    verified_at: Option<DateTime<Utc>>,
    sso_required: bool,
    sso_provider_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct TrustHostingRegionRow {
    data_region: String,
    legal_jurisdiction: String,
    workspace_count: i64,
}

#[derive(sqlx::FromRow)]
struct TrustAuditEventRow {
    id: Uuid,
    event_type: String,
    actor_id: Option<Uuid>,
    actor_email: Option<String>,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}
