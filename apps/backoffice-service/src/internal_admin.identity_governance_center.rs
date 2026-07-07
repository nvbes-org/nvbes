use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;
use crate::identity_governance_operator_grants::{OperatorGrant, OperatorRoleDistribution};

#[derive(Debug, Serialize)]
struct IdentityGovernanceSnapshot {
    active_idp_count: i64,
    unverified_domain_count: i64,
    active_scim_connector_count: i64,
    overdue_access_review_count: i64,
    pending_review_item_count: i64,
    active_break_glass_count: i64,
    pending_recovery_count: i64,
    active_operator_grant_count: i64,
    revoked_operator_grant_count: i64,
    unverified_domains: Vec<UnverifiedDomain>,
    sso_providers: Vec<SsoProvider>,
    scim_connectors: Vec<ScimConnector>,
    overdue_access_reviews: Vec<OverdueAccessReview>,
    break_glass_accounts: Vec<BreakGlassAccount>,
    pending_recovery_requests: Vec<PendingRecoveryRequest>,
    operator_role_distribution: Vec<OperatorRoleDistribution>,
    operator_grants: Vec<OperatorGrant>,
}

#[derive(Debug, Serialize)]
struct UnverifiedDomain {
    tenant_id: Uuid,
    tenant_name: String,
    domain: String,
    created_at: DateTime<Utc>,
    verification_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct SsoProvider {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    provider_type: String,
    name: String,
    status: String,
    issuer: Option<String>,
    require_signed_assertions: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ScimConnector {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    provider: String,
    status: String,
    base_url: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct OverdueAccessReview {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    name: String,
    status: String,
    due_at: DateTime<Utc>,
    pending_item_count: i64,
}

#[derive(Debug, Serialize)]
struct BreakGlassAccount {
    tenant_id: Uuid,
    tenant_name: String,
    principal_id: Uuid,
    procedure_reference: String,
    reason: String,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PendingRecoveryRequest {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    principal_id: Uuid,
    email: String,
    status: String,
    available_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/identity-governance-center",
            get(identity_governance_center_route),
        )
        .merge(crate::identity_governance_center_actions::router())
        .merge(crate::identity_governance_operator_grant_actions::router())
}

async fn identity_governance_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<IdentityGovernanceSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_identity_governance(&state.db).await?))
}

async fn load_identity_governance(db: &PgPool) -> Result<IdentityGovernanceSnapshot, AppError> {
    let operator_grants =
        crate::identity_governance_operator_grants::load_operator_grants(db).await?;
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM federated_identity_providers WHERE status = 'active')
            AS active_idp_count,
          (SELECT COUNT(*) FROM tenant_domains WHERE verified_at IS NULL)
            AS unverified_domain_count,
          (SELECT COUNT(*) FROM scim_provisioning_connectors WHERE status = 'active')
            AS active_scim_connector_count,
          (
            SELECT COUNT(*) FROM access_review_campaigns
            WHERE status::text = 'active' AND due_at < NOW()
          ) AS overdue_access_review_count,
          (
            SELECT COUNT(*) FROM access_review_items
            WHERE decision::text = 'pending'
          ) AS pending_review_item_count,
          (
            SELECT COUNT(*) FROM tenant_break_glass_accounts
            WHERE revoked_at IS NULL
          ) AS active_break_glass_count,
          (
            SELECT COUNT(*) FROM enterprise_password_recovery_requests
            WHERE status = 'pending'
          ) AS pending_recovery_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(IdentityGovernanceSnapshot {
        active_idp_count: metrics.get("active_idp_count"),
        unverified_domain_count: metrics.get("unverified_domain_count"),
        active_scim_connector_count: metrics.get("active_scim_connector_count"),
        overdue_access_review_count: metrics.get("overdue_access_review_count"),
        pending_review_item_count: metrics.get("pending_review_item_count"),
        active_break_glass_count: metrics.get("active_break_glass_count"),
        pending_recovery_count: metrics.get("pending_recovery_count"),
        active_operator_grant_count: operator_grants.active_count,
        revoked_operator_grant_count: operator_grants.revoked_count,
        unverified_domains: load_unverified_domains(db).await?,
        sso_providers: load_sso_providers(db).await?,
        scim_connectors: load_scim_connectors(db).await?,
        overdue_access_reviews: load_overdue_access_reviews(db).await?,
        break_glass_accounts: load_break_glass_accounts(db).await?,
        pending_recovery_requests: load_pending_recovery_requests(db).await?,
        operator_role_distribution: operator_grants.role_distribution,
        operator_grants: operator_grants.grants,
    })
}

async fn load_unverified_domains(db: &PgPool) -> Result<Vec<UnverifiedDomain>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT td.tenant_id, t.name AS tenant_name, td.domain, td.created_at,
          td.verification_expires_at
        FROM tenant_domains td
        JOIN tenants t ON t.id = td.tenant_id
        WHERE td.verified_at IS NULL
        ORDER BY td.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UnverifiedDomain {
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            domain: row.get("domain"),
            created_at: row.get("created_at"),
            verification_expires_at: row.get("verification_expires_at"),
        })
        .collect())
}

async fn load_sso_providers(db: &PgPool) -> Result<Vec<SsoProvider>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT fip.id, fip.tenant_id, t.name AS tenant_name, fip.provider_type::text AS provider_type,
          fip.name, fip.status, fip.issuer, fip.require_signed_assertions, fip.created_at
        FROM federated_identity_providers fip
        JOIN tenants t ON t.id = fip.tenant_id
        ORDER BY fip.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| SsoProvider {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            provider_type: row.get("provider_type"),
            name: row.get("name"),
            status: row.get("status"),
            issuer: row.get("issuer"),
            require_signed_assertions: row.get("require_signed_assertions"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_scim_connectors(db: &PgPool) -> Result<Vec<ScimConnector>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT sc.id, sc.tenant_id, t.name AS tenant_name, sc.provider, sc.status,
          sc.base_url, sc.created_at
        FROM scim_provisioning_connectors sc
        JOIN tenants t ON t.id = sc.tenant_id
        ORDER BY sc.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ScimConnector {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            provider: row.get("provider"),
            status: row.get("status"),
            base_url: row.get("base_url"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_overdue_access_reviews(db: &PgPool) -> Result<Vec<OverdueAccessReview>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT arc.id, arc.tenant_id, t.name AS tenant_name, arc.name,
          arc.status::text AS status, arc.due_at,
          COUNT(ari.id) FILTER (WHERE ari.decision::text = 'pending') AS pending_item_count
        FROM access_review_campaigns arc
        JOIN tenants t ON t.id = arc.tenant_id
        LEFT JOIN access_review_items ari ON ari.campaign_id = arc.id
        WHERE arc.status::text = 'active' AND arc.due_at < NOW()
        GROUP BY arc.id, arc.tenant_id, t.name, arc.name, arc.status, arc.due_at
        ORDER BY arc.due_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OverdueAccessReview {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            name: row.get("name"),
            status: row.get("status"),
            due_at: row.get("due_at"),
            pending_item_count: row.get("pending_item_count"),
        })
        .collect())
}

async fn load_break_glass_accounts(db: &PgPool) -> Result<Vec<BreakGlassAccount>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bga.tenant_id, t.name AS tenant_name, bga.principal_id,
          bga.procedure_reference, bga.reason, bga.last_used_at, bga.created_at
        FROM tenant_break_glass_accounts bga
        JOIN tenants t ON t.id = bga.tenant_id
        WHERE bga.revoked_at IS NULL
        ORDER BY bga.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BreakGlassAccount {
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            principal_id: row.get("principal_id"),
            procedure_reference: row.get("procedure_reference"),
            reason: row.get("reason"),
            last_used_at: row.get("last_used_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_pending_recovery_requests(
    db: &PgPool,
) -> Result<Vec<PendingRecoveryRequest>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT epr.id, epr.tenant_id, t.name AS tenant_name, epr.principal_id,
          epr.email, epr.status, epr.available_at, epr.created_at
        FROM enterprise_password_recovery_requests epr
        JOIN tenants t ON t.id = epr.tenant_id
        WHERE epr.status = 'pending'
        ORDER BY epr.available_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingRecoveryRequest {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            status: row.get("status"),
            available_at: row.get("available_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}
