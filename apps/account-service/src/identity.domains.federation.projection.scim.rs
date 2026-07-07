use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::federation::{
        projection::{empty_to_option, parse_required_time, parse_required_uuid},
        types::ScimProvisioningConnectorRecord,
    },
    grpc_pb::nvbes::enterprise::v1 as enterprise,
    http::error::AppError,
};

pub async fn upsert_scim_connector_projection(
    db: &PgPool,
    connector: &enterprise::ScimConnector,
) -> Result<ScimProvisioningConnectorRecord, AppError> {
    let connector_id = parse_required_uuid(&connector.connector_id, "connector_id")?;
    let tenant_id = parse_required_uuid(&connector.tenant_id, "tenant_id")?;
    let created_at = parse_required_time(&connector.created_at, "created_at")?;

    let row = sqlx::query_as::<_, ScimProvisioningConnectorRecord>(
        r#"
        INSERT INTO scim_provisioning_connectors (
          id,
          tenant_id,
          provider,
          status,
          base_url,
          created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id)
        DO UPDATE SET
          provider = EXCLUDED.provider,
          status = EXCLUDED.status,
          base_url = EXCLUDED.base_url,
          created_at = EXCLUDED.created_at
        WHERE scim_provisioning_connectors.tenant_id = EXCLUDED.tenant_id
        RETURNING id, provider, status, base_url, created_at
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .bind(&connector.provider)
    .bind(&connector.status)
    .bind(empty_to_option(&connector.base_url))
    .bind(created_at)
    .fetch_one(db)
    .await?;

    Ok(row)
}

pub async fn delete_scim_connector_projection(
    db: &PgPool,
    tenant_id: Uuid,
    connector_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM scim_provisioning_connectors
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .execute(db)
    .await?;

    Ok(())
}
