use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

pub async fn list_developer_roles(
    db: &sqlx::PgPool,
    request: developer::ListDeveloperRolesRequest,
) -> Result<developer::ListDeveloperRolesResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    Ok(developer::ListDeveloperRolesResponse {
        roles: list_roles(db, tenant_id, principal_id).await?,
    })
}

async fn list_roles(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Vec<String>, Status> {
    sqlx::query_scalar(
        r#"
        SELECT role::text
        FROM developer_role_assignments
        WHERE tenant_id = $1
          AND principal_id = $2
          AND revoked_at IS NULL
        ORDER BY role::text
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
}

#[cfg(test)]
#[path = "developer.grpc.rbac.contract_tests.rs"]
mod contract_tests;
