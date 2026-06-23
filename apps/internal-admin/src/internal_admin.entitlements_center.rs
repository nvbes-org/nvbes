use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct EntitlementsSnapshot {
    active_plan_count: i64,
    active_feature_count: i64,
    quota_definition_count: i64,
    active_entitlement_count: i64,
    over_quota_balance_count: i64,
    unpublished_change_count: i64,
    active_trial_grant_count: i64,
    active_plans: Vec<EntitlementPlan>,
    over_quota_balances: Vec<OverQuotaBalance>,
    expiring_entitlements: Vec<ExpiringEntitlement>,
    unpublished_changes: Vec<UnpublishedEntitlementChange>,
}

#[derive(Debug, Serialize)]
struct EntitlementPlan {
    plan_id: Uuid,
    product_name: String,
    plan_code: String,
    plan_name: String,
    active_version_count: i64,
    feature_count: i64,
}

#[derive(Debug, Serialize)]
struct OverQuotaBalance {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    quota_code: String,
    included_quantity: i64,
    used_quantity: i64,
    period_end: NaiveDate,
}

#[derive(Debug, Serialize)]
struct ExpiringEntitlement {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    status: String,
    effective_to: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct UnpublishedEntitlementChange {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    event_id: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/entitlements-center", get(entitlements_center_route))
}

async fn entitlements_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<EntitlementsSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_entitlements(&state.db).await?))
}

async fn load_entitlements(db: &PgPool) -> Result<EntitlementsSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM billing_plans WHERE status = 'active') AS active_plan_count,
          (SELECT COUNT(*) FROM billing_features) AS active_feature_count,
          (SELECT COUNT(*) FROM billing_quota_definitions) AS quota_definition_count,
          (
            SELECT COUNT(*) FROM billing_entitlement_snapshots
            WHERE status = 'active' AND (effective_to IS NULL OR effective_to > NOW())
          ) AS active_entitlement_count,
          (
            SELECT COUNT(*) FROM billing_quota_balances
            WHERE used_quantity > included_quantity
          ) AS over_quota_balance_count,
          (
            SELECT COUNT(*) FROM billing_entitlement_changes
            WHERE published_at IS NULL
          ) AS unpublished_change_count,
          (
            SELECT COUNT(*) FROM billing_trial_grants
            WHERE starts_at <= NOW() AND ends_at > NOW()
          ) AS active_trial_grant_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(EntitlementsSnapshot {
        active_plan_count: metrics.get("active_plan_count"),
        active_feature_count: metrics.get("active_feature_count"),
        quota_definition_count: metrics.get("quota_definition_count"),
        active_entitlement_count: metrics.get("active_entitlement_count"),
        over_quota_balance_count: metrics.get("over_quota_balance_count"),
        unpublished_change_count: metrics.get("unpublished_change_count"),
        active_trial_grant_count: metrics.get("active_trial_grant_count"),
        active_plans: load_active_plans(db).await?,
        over_quota_balances: load_over_quota_balances(db).await?,
        expiring_entitlements: load_expiring_entitlements(db).await?,
        unpublished_changes: load_unpublished_changes(db).await?,
    })
}

async fn load_active_plans(db: &PgPool) -> Result<Vec<EntitlementPlan>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bp.id AS plan_id, pr.name AS product_name, bp.code AS plan_code,
          bp.name AS plan_name, COUNT(DISTINCT bpv.id) AS active_version_count,
          COUNT(DISTINCT bpf.feature_id) AS feature_count
        FROM billing_plans bp
        JOIN billing_products pr ON pr.id = bp.product_id
        LEFT JOIN billing_plan_versions bpv ON bpv.plan_id = bp.id AND bpv.status = 'active'
        LEFT JOIN billing_plan_features bpf ON bpf.plan_version_id = bpv.id AND bpf.enabled = TRUE
        WHERE bp.status = 'active'
        GROUP BY bp.id, pr.name, bp.code, bp.name
        ORDER BY pr.name ASC, bp.name ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EntitlementPlan {
            plan_id: row.get("plan_id"),
            product_name: row.get("product_name"),
            plan_code: row.get("plan_code"),
            plan_name: row.get("plan_name"),
            active_version_count: row.get("active_version_count"),
            feature_count: row.get("feature_count"),
        })
        .collect())
}

async fn load_over_quota_balances(db: &PgPool) -> Result<Vec<OverQuotaBalance>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT qb.id, qb.tenant_id, t.name AS tenant_name, qb.workspace_id, w.name AS workspace_name,
          qb.quota_code, qb.included_quantity, qb.used_quantity, qb.period_end
        FROM billing_quota_balances qb
        JOIN tenants t ON t.id = qb.tenant_id
        LEFT JOIN workspaces w ON w.id = qb.workspace_id
        WHERE qb.used_quantity > qb.included_quantity
        ORDER BY (qb.used_quantity - qb.included_quantity) DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OverQuotaBalance {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            quota_code: row.get("quota_code"),
            included_quantity: row.get("included_quantity"),
            used_quantity: row.get("used_quantity"),
            period_end: row.get("period_end"),
        })
        .collect())
}

async fn load_expiring_entitlements(db: &PgPool) -> Result<Vec<ExpiringEntitlement>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT es.id, es.tenant_id, t.name AS tenant_name, es.workspace_id, w.name AS workspace_name,
          es.status, es.effective_to
        FROM billing_entitlement_snapshots es
        JOIN tenants t ON t.id = es.tenant_id
        LEFT JOIN workspaces w ON w.id = es.workspace_id
        WHERE es.effective_to IS NOT NULL
          AND es.effective_to > NOW()
          AND es.effective_to <= NOW() + INTERVAL '14 days'
        ORDER BY es.effective_to ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ExpiringEntitlement {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            status: row.get("status"),
            effective_to: row.get("effective_to"),
        })
        .collect())
}

async fn load_unpublished_changes(
    db: &PgPool,
) -> Result<Vec<UnpublishedEntitlementChange>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ec.id, ec.tenant_id, t.name AS tenant_name, ec.event_id, ec.created_at
        FROM billing_entitlement_changes ec
        JOIN tenants t ON t.id = ec.tenant_id
        WHERE ec.published_at IS NULL
        ORDER BY ec.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UnpublishedEntitlementChange {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            event_id: row.get("event_id"),
            created_at: row.get("created_at"),
        })
        .collect())
}
