use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct RegionCenterSnapshot {
    eu_workspace_count: i64,
    non_eu_workspace_count: i64,
    gdpr_workspace_count: i64,
    non_gdpr_workspace_count: i64,
    multi_region_tenant_count: i64,
    region_distribution: Vec<RegionDistribution>,
    jurisdiction_distribution: Vec<JurisdictionDistribution>,
    non_eu_workspaces: Vec<RegionWorkspace>,
    multi_region_tenants: Vec<MultiRegionTenant>,
}

#[derive(Debug, Serialize)]
struct RegionDistribution {
    data_region: String,
    workspace_count: i64,
    tenant_count: i64,
}

#[derive(Debug, Serialize)]
struct JurisdictionDistribution {
    jurisdiction: String,
    workspace_count: i64,
    tenant_count: i64,
}

#[derive(Debug, Serialize)]
struct RegionWorkspace {
    workspace_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_name: String,
    data_region: String,
    jurisdiction: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MultiRegionTenant {
    tenant_id: Uuid,
    tenant_name: String,
    workspace_count: i64,
    region_count: i64,
    regions: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/region-center", get(region_center_route))
}

async fn region_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RegionCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_region_center(&state.db).await?))
}

async fn load_region_center(db: &PgPool) -> Result<RegionCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM workspaces
            WHERE data_region::text = 'eu'
          ) AS eu_workspace_count,
          (
            SELECT COUNT(*) FROM workspaces
            WHERE data_region::text <> 'eu'
          ) AS non_eu_workspace_count,
          (
            SELECT COUNT(*) FROM workspaces
            WHERE jurisdiction::text = 'gdpr'
          ) AS gdpr_workspace_count,
          (
            SELECT COUNT(*) FROM workspaces
            WHERE jurisdiction::text <> 'gdpr'
          ) AS non_gdpr_workspace_count,
          (
            SELECT COUNT(*) FROM (
              SELECT tenant_id
              FROM workspaces
              GROUP BY tenant_id
              HAVING COUNT(DISTINCT data_region::text) > 1
            ) tenants
          ) AS multi_region_tenant_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(RegionCenterSnapshot {
        eu_workspace_count: metrics.get("eu_workspace_count"),
        non_eu_workspace_count: metrics.get("non_eu_workspace_count"),
        gdpr_workspace_count: metrics.get("gdpr_workspace_count"),
        non_gdpr_workspace_count: metrics.get("non_gdpr_workspace_count"),
        multi_region_tenant_count: metrics.get("multi_region_tenant_count"),
        region_distribution: load_region_distribution(db).await?,
        jurisdiction_distribution: load_jurisdiction_distribution(db).await?,
        non_eu_workspaces: load_non_eu_workspaces(db).await?,
        multi_region_tenants: load_multi_region_tenants(db).await?,
    })
}

async fn load_region_distribution(db: &PgPool) -> Result<Vec<RegionDistribution>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT data_region::text AS data_region, COUNT(*) AS workspace_count,
          COUNT(DISTINCT tenant_id) AS tenant_count
        FROM workspaces
        GROUP BY data_region
        ORDER BY workspace_count DESC, data_region ASC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RegionDistribution {
            data_region: row.get("data_region"),
            workspace_count: row.get("workspace_count"),
            tenant_count: row.get("tenant_count"),
        })
        .collect())
}

async fn load_jurisdiction_distribution(
    db: &PgPool,
) -> Result<Vec<JurisdictionDistribution>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT jurisdiction::text AS jurisdiction, COUNT(*) AS workspace_count,
          COUNT(DISTINCT tenant_id) AS tenant_count
        FROM workspaces
        GROUP BY jurisdiction
        ORDER BY workspace_count DESC, jurisdiction ASC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| JurisdictionDistribution {
            jurisdiction: row.get("jurisdiction"),
            workspace_count: row.get("workspace_count"),
            tenant_count: row.get("tenant_count"),
        })
        .collect())
}

async fn load_non_eu_workspaces(db: &PgPool) -> Result<Vec<RegionWorkspace>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT w.id AS workspace_id, w.tenant_id, t.name AS tenant_name,
          w.name AS workspace_name, w.data_region::text AS data_region,
          w.jurisdiction::text AS jurisdiction, w.created_at
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        WHERE w.data_region::text <> 'eu'
        ORDER BY w.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RegionWorkspace {
            workspace_id: row.get("workspace_id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_name: row.get("workspace_name"),
            data_region: row.get("data_region"),
            jurisdiction: row.get("jurisdiction"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_multi_region_tenants(db: &PgPool) -> Result<Vec<MultiRegionTenant>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT t.id AS tenant_id, t.name AS tenant_name, COUNT(*) AS workspace_count,
          COUNT(DISTINCT w.data_region::text) AS region_count,
          ARRAY_AGG(DISTINCT w.data_region::text ORDER BY w.data_region::text) AS regions
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        GROUP BY t.id, t.name
        HAVING COUNT(DISTINCT w.data_region::text) > 1
        ORDER BY region_count DESC, workspace_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MultiRegionTenant {
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_count: row.get("workspace_count"),
            region_count: row.get("region_count"),
            regions: row.get("regions"),
        })
        .collect())
}
