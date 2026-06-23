use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct AccessCenterSnapshot {
    workspace_owner_count: i64,
    workspace_admin_count: i64,
    ownerless_workspace_count: i64,
    service_account_count: i64,
    stale_service_account_count: i64,
    oauth_client_count: i64,
    revoked_oauth_client_count: i64,
    restricted_client_policy_count: i64,
    privileged_users: Vec<PrivilegedUser>,
    ownerless_workspaces: Vec<OwnerlessWorkspace>,
    stale_service_accounts: Vec<StaleServiceAccount>,
}

#[derive(Debug, Serialize)]
struct PrivilegedUser {
    principal_id: Uuid,
    email: Option<String>,
    name: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Uuid,
    workspace_name: String,
    role: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct OwnerlessWorkspace {
    workspace_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_name: String,
    plan_code: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StaleServiceAccount {
    principal_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    name: String,
    last_rotated_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/access-center", get(access_center_route))
        .merge(crate::access_center_actions::router())
}

async fn access_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AccessCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_access_center(&state.db).await?))
}

async fn load_access_center(db: &PgPool) -> Result<AccessCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM workspace_memberships
            WHERE status::text = 'active' AND role::text = 'owner'
          ) AS workspace_owner_count,
          (
            SELECT COUNT(*) FROM workspace_memberships
            WHERE status::text = 'active' AND role::text = 'admin'
          ) AS workspace_admin_count,
          (
            SELECT COUNT(*) FROM workspaces w
            WHERE NOT EXISTS (
              SELECT 1 FROM workspace_memberships wm
              WHERE wm.workspace_id = w.id
                AND wm.status::text = 'active'
                AND wm.role::text = 'owner'
            )
          ) AS ownerless_workspace_count,
          (SELECT COUNT(*) FROM service_accounts) AS service_account_count,
          (
            SELECT COUNT(*) FROM service_accounts
            WHERE last_rotated_at IS NULL OR last_rotated_at < NOW() - INTERVAL '90 days'
          ) AS stale_service_account_count,
          (
            SELECT COUNT(*) FROM oauth_clients
            WHERE revoked_at IS NULL
          ) AS oauth_client_count,
          (
            SELECT COUNT(*) FROM oauth_clients
            WHERE revoked_at IS NOT NULL
          ) AS revoked_oauth_client_count,
          (
            SELECT COUNT(*) FROM oauth_client_policies
            WHERE status::text IN ('restricted', 'blocked', 'pending_approval')
          ) AS restricted_client_policy_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(AccessCenterSnapshot {
        workspace_owner_count: metrics.get("workspace_owner_count"),
        workspace_admin_count: metrics.get("workspace_admin_count"),
        ownerless_workspace_count: metrics.get("ownerless_workspace_count"),
        service_account_count: metrics.get("service_account_count"),
        stale_service_account_count: metrics.get("stale_service_account_count"),
        oauth_client_count: metrics.get("oauth_client_count"),
        revoked_oauth_client_count: metrics.get("revoked_oauth_client_count"),
        restricted_client_policy_count: metrics.get("restricted_client_policy_count"),
        privileged_users: load_privileged_users(db).await?,
        ownerless_workspaces: load_ownerless_workspaces(db).await?,
        stale_service_accounts: load_stale_service_accounts(db).await?,
    })
}

async fn load_privileged_users(db: &PgPool) -> Result<Vec<PrivilegedUser>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT wm.principal_id, u.email, u.name, w.tenant_id, t.name AS tenant_name,
          w.id AS workspace_id, w.name AS workspace_name, wm.role::text AS role, wm.updated_at
        FROM workspace_memberships wm
        JOIN workspaces w ON w.id = wm.workspace_id
        JOIN tenants t ON t.id = w.tenant_id
        LEFT JOIN users u ON u.principal_id = wm.principal_id
        WHERE wm.status::text = 'active' AND wm.role::text IN ('owner', 'admin')
        ORDER BY wm.updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PrivilegedUser {
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            name: row.get("name"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            role: row.get("role"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_ownerless_workspaces(db: &PgPool) -> Result<Vec<OwnerlessWorkspace>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT w.id AS workspace_id, w.tenant_id, t.name AS tenant_name,
          w.name AS workspace_name, w.plan_code, w.created_at
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        WHERE NOT EXISTS (
          SELECT 1 FROM workspace_memberships wm
          WHERE wm.workspace_id = w.id
            AND wm.status::text = 'active'
            AND wm.role::text = 'owner'
        )
        ORDER BY w.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OwnerlessWorkspace {
            workspace_id: row.get("workspace_id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_name: row.get("workspace_name"),
            plan_code: row.get("plan_code"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_stale_service_accounts(db: &PgPool) -> Result<Vec<StaleServiceAccount>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT sa.principal_id, sa.tenant_id, t.name AS tenant_name, sa.workspace_id,
          w.name AS workspace_name, sa.name, sa.last_rotated_at, sa.created_at
        FROM service_accounts sa
        JOIN tenants t ON t.id = sa.tenant_id
        LEFT JOIN workspaces w ON w.id = sa.workspace_id
        WHERE sa.last_rotated_at IS NULL OR sa.last_rotated_at < NOW() - INTERVAL '90 days'
        ORDER BY sa.last_rotated_at ASC NULLS FIRST, sa.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| StaleServiceAccount {
            principal_id: row.get("principal_id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            name: row.get("name"),
            last_rotated_at: row.get("last_rotated_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}
