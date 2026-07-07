use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

pub async fn overview_summary(
    db: &sqlx::PgPool,
    request: developer::GetOverviewSummaryRequest,
) -> Result<developer::DeveloperOverviewSummary, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let (
        oauth_clients,
        marketplace_pending,
        high_risk_scopes,
        failed_webhook_deliveries,
        unhealthy_integrations,
        active_sandboxes,
    ) = tokio::try_join!(
        count_oauth_clients(db, tenant_id),
        count_pending_marketplace_apps(db, tenant_id),
        count_high_risk_scopes(db),
        count_failed_webhook_deliveries(db, tenant_id),
        count_unhealthy_integrations(db, tenant_id),
        count_active_sandboxes(db, tenant_id),
    )?;

    Ok(developer::DeveloperOverviewSummary {
        tenant_id: tenant_id.to_string(),
        oauth_clients,
        marketplace_pending,
        high_risk_scopes,
        failed_webhook_deliveries,
        unhealthy_integrations,
        active_sandboxes,
    })
}

async fn count_oauth_clients(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<i64, Status> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM oauth_clients WHERE tenant_id = $1",
    )
    .await
}

async fn count_pending_marketplace_apps(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<i64, Status> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_marketplace_apps WHERE tenant_id = $1 AND status = 'pending'",
    )
    .await
}

async fn count_high_risk_scopes(db: &sqlx::PgPool) -> Result<i64, Status> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM developer_scope_registry WHERE risk IN ('high', 'restricted')",
    )
    .fetch_one(db)
    .await
    .map_err(sql_status)
}

async fn count_failed_webhook_deliveries(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<i64, Status> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_webhook_deliveries WHERE tenant_id = $1 AND status = 'failed'",
    )
    .await
}

async fn count_unhealthy_integrations(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<i64, Status> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_health_checks WHERE tenant_id = $1 AND status IN ('failing', 'warning')",
    )
    .await
}

async fn count_active_sandboxes(db: &sqlx::PgPool, tenant_id: Uuid) -> Result<i64, Status> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_sandbox_tenants WHERE tenant_id = $1 AND status = 'active'",
    )
    .await
}

async fn count(db: &sqlx::PgPool, tenant_id: Uuid, sql: &str) -> Result<i64, Status> {
    sqlx::query_scalar(sql)
        .bind(tenant_id)
        .fetch_one(db)
        .await
        .map_err(sql_status)
}

#[cfg(test)]
#[path = "developer.grpc.overview.contract_tests.rs"]
mod contract_tests;
