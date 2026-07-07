use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    federation::{empty_to_option, time_string},
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, optional_uuid, sql_status},
};

pub async fn configure_scim_connector(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: enterprise::ConfigureScimConnectorRequest,
) -> Result<enterprise::ScimConnector, Status> {
    let connector_id =
        optional_uuid(&request.connector_id, "connector_id")?.unwrap_or_else(Uuid::new_v4);
    let provider = non_empty(request.provider, "provider")?;

    let row = sqlx::query(
        r#"
        INSERT INTO scim_provisioning_connectors (
          id,
          tenant_id,
          provider,
          status,
          base_url
        )
        VALUES ($1, $2, $3, $4, NULLIF($5, ''))
        ON CONFLICT (id)
        DO UPDATE SET
          provider = EXCLUDED.provider,
          status = EXCLUDED.status,
          base_url = EXCLUDED.base_url
        WHERE scim_provisioning_connectors.tenant_id = EXCLUDED.tenant_id
        RETURNING id, tenant_id, provider, status, base_url, created_at
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .bind(provider)
    .bind(default_status(&request.status))
    .bind(request.base_url.trim())
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise SCIM connector was not found"))?;

    Ok(connector_from_row(row))
}

pub async fn delete_scim_connector(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    connector_id: Uuid,
) -> Result<enterprise::ScimConnectorDeletion, Status> {
    let row = sqlx::query(
        r#"
        DELETE FROM scim_provisioning_connectors
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::ScimConnectorDeletion {
        connector_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
    })
}

pub(crate) fn connector_from_row(row: PgRow) -> enterprise::ScimConnector {
    enterprise::ScimConnector {
        connector_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        provider: row.get("provider"),
        status: row.get("status"),
        base_url: row.get::<Option<String>, _>("base_url").unwrap_or_default(),
        created_at: time_string(row.get("created_at")),
    }
}

fn default_status(value: &str) -> String {
    empty_to_option(value).unwrap_or_else(|| "active".to_string())
}
