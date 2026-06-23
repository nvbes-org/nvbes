use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct DeveloperCenterSnapshot {
    active_client_count: i64,
    pending_marketplace_app_count: i64,
    failed_webhook_delivery_count_24h: i64,
    active_webhook_endpoint_count: i64,
    expiring_secret_count: i64,
    restricted_scope_count: i64,
    failing_health_check_count: i64,
    pending_marketplace_apps: Vec<PendingMarketplaceApp>,
    webhook_failures: Vec<WebhookFailure>,
    expiring_secrets: Vec<ExpiringSecret>,
    risky_scopes: Vec<RiskyScope>,
    health_issues: Vec<DeveloperHealthIssue>,
}

#[derive(Debug, Serialize)]
struct PendingMarketplaceApp {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    client_id: String,
    client_name: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WebhookFailure {
    id: Uuid,
    endpoint_id: Uuid,
    endpoint_name: String,
    tenant_id: Uuid,
    tenant_name: String,
    event_type: String,
    status: String,
    attempt_count: i32,
    response_status: Option<i32>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ExpiringSecret {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    client_id: String,
    client_name: String,
    status: String,
    secret_last4: String,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct RiskyScope {
    scope_key: String,
    display_name: String,
    risk: String,
    lifecycle: String,
    owner_team: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct DeveloperHealthIssue {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    target_type: String,
    target_id: String,
    check_kind: String,
    status: String,
    summary: String,
    checked_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/developer-center", get(developer_center_route))
}

async fn developer_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DeveloperCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_developer_center(&state.db).await?))
}

async fn load_developer_center(db: &PgPool) -> Result<DeveloperCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM oauth_clients WHERE revoked_at IS NULL) AS active_client_count,
          (SELECT COUNT(*) FROM developer_marketplace_apps WHERE status::text = 'pending')
            AS pending_marketplace_app_count,
          (
            SELECT COUNT(*) FROM developer_webhook_deliveries
            WHERE status::text = 'failed' AND created_at >= NOW() - INTERVAL '24 hours'
          ) AS failed_webhook_delivery_count_24h,
          (
            SELECT COUNT(*) FROM developer_webhook_endpoints
            WHERE status::text = 'active' AND revoked_at IS NULL
          ) AS active_webhook_endpoint_count,
          (
            SELECT COUNT(*) FROM developer_client_secret_versions
            WHERE status::text IN ('active', 'overlap')
              AND revoked_at IS NULL
              AND expires_at IS NOT NULL
              AND expires_at <= NOW() + INTERVAL '14 days'
          ) AS expiring_secret_count,
          (
            SELECT COUNT(*) FROM developer_scope_registry
            WHERE risk::text = 'restricted' AND lifecycle::text IN ('proposed', 'active')
          ) AS restricted_scope_count,
          (
            SELECT COUNT(*) FROM developer_health_checks
            WHERE status::text IN ('warning', 'failing')
          ) AS failing_health_check_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(DeveloperCenterSnapshot {
        active_client_count: metrics.get("active_client_count"),
        pending_marketplace_app_count: metrics.get("pending_marketplace_app_count"),
        failed_webhook_delivery_count_24h: metrics.get("failed_webhook_delivery_count_24h"),
        active_webhook_endpoint_count: metrics.get("active_webhook_endpoint_count"),
        expiring_secret_count: metrics.get("expiring_secret_count"),
        restricted_scope_count: metrics.get("restricted_scope_count"),
        failing_health_check_count: metrics.get("failing_health_check_count"),
        pending_marketplace_apps: load_pending_marketplace_apps(db).await?,
        webhook_failures: load_webhook_failures(db).await?,
        expiring_secrets: load_expiring_secrets(db).await?,
        risky_scopes: load_risky_scopes(db).await?,
        health_issues: load_health_issues(db).await?,
    })
}

async fn load_pending_marketplace_apps(
    db: &PgPool,
) -> Result<Vec<PendingMarketplaceApp>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ma.id, ma.tenant_id, t.name AS tenant_name, ma.client_id,
          oc.name AS client_name, ma.status::text AS status, ma.created_at, ma.updated_at
        FROM developer_marketplace_apps ma
        JOIN tenants t ON t.id = ma.tenant_id
        JOIN oauth_clients oc ON oc.tenant_id = ma.tenant_id AND oc.client_id = ma.client_id
        WHERE ma.status::text = 'pending'
        ORDER BY ma.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingMarketplaceApp {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_webhook_failures(db: &PgPool) -> Result<Vec<WebhookFailure>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT wd.id, wd.endpoint_id, we.name AS endpoint_name, wd.tenant_id,
          t.name AS tenant_name, wd.event_type, wd.status::text AS status,
          wd.attempt_count, wd.response_status, wd.error_message, wd.created_at
        FROM developer_webhook_deliveries wd
        JOIN developer_webhook_endpoints we ON we.tenant_id = wd.tenant_id AND we.id = wd.endpoint_id
        JOIN tenants t ON t.id = wd.tenant_id
        WHERE wd.status::text = 'failed'
        ORDER BY wd.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| WebhookFailure {
            id: row.get("id"),
            endpoint_id: row.get("endpoint_id"),
            endpoint_name: row.get("endpoint_name"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            event_type: row.get("event_type"),
            status: row.get("status"),
            attempt_count: row.get("attempt_count"),
            response_status: row.get("response_status"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_expiring_secrets(db: &PgPool) -> Result<Vec<ExpiringSecret>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT sv.id, sv.tenant_id, t.name AS tenant_name, sv.client_id,
          oc.name AS client_name, sv.status::text AS status, sv.secret_last4, sv.expires_at
        FROM developer_client_secret_versions sv
        JOIN tenants t ON t.id = sv.tenant_id
        JOIN oauth_clients oc ON oc.tenant_id = sv.tenant_id AND oc.client_id = sv.client_id
        WHERE sv.status::text IN ('active', 'overlap')
          AND sv.revoked_at IS NULL
          AND sv.expires_at IS NOT NULL
          AND sv.expires_at <= NOW() + INTERVAL '14 days'
        ORDER BY sv.expires_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ExpiringSecret {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            status: row.get("status"),
            secret_last4: row.get("secret_last4"),
            expires_at: row.get("expires_at"),
        })
        .collect())
}

async fn load_risky_scopes(db: &PgPool) -> Result<Vec<RiskyScope>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT scope_key, display_name, risk::text AS risk, lifecycle::text AS lifecycle,
          owner_team, updated_at
        FROM developer_scope_registry
        WHERE risk::text IN ('high', 'restricted') AND lifecycle::text IN ('proposed', 'active')
        ORDER BY risk DESC, updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RiskyScope {
            scope_key: row.get("scope_key"),
            display_name: row.get("display_name"),
            risk: row.get("risk"),
            lifecycle: row.get("lifecycle"),
            owner_team: row.get("owner_team"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_health_issues(db: &PgPool) -> Result<Vec<DeveloperHealthIssue>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT hc.id, hc.tenant_id, t.name AS tenant_name, hc.target_type, hc.target_id,
          hc.check_kind, hc.status::text AS status, hc.summary, hc.checked_at
        FROM developer_health_checks hc
        JOIN tenants t ON t.id = hc.tenant_id
        WHERE hc.status::text IN ('warning', 'failing')
        ORDER BY hc.checked_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| DeveloperHealthIssue {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            check_kind: row.get("check_kind"),
            status: row.get("status"),
            summary: row.get("summary"),
            checked_at: row.get("checked_at"),
        })
        .collect())
}
