use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::federation::{
        contract::{normalize_scim_base_url, normalize_scim_provider},
        types::{
            CreateScimProvisioningConnectorInput, ScimProvisioningConnectorRecord,
            ScimProvisioningConnectorResponse, ScimProvisioningConnectorsResponse,
            UpdateScimProvisioningConnectorInput,
        },
        validation::normalize_registry_status,
    },
    http::error::AppError,
};

pub async fn list_scim_connectors(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<ScimProvisioningConnectorsResponse, AppError> {
    let rows = sqlx::query_as::<_, ScimProvisioningConnectorRecord>(
        r#"
        SELECT
          id,
          provider,
          status,
          base_url,
          created_at
        FROM scim_provisioning_connectors
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(ScimProvisioningConnectorsResponse {
        connectors: rows
            .into_iter()
            .map(ScimProvisioningConnectorRecord::into_view)
            .collect(),
    })
}

pub async fn create_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    input: CreateScimProvisioningConnectorInput,
) -> Result<ScimProvisioningConnectorResponse, AppError> {
    let provider = normalize_scim_provider(&input.provider)?;
    let status = normalize_registry_status(input.status.as_deref())?;
    let base_url = normalize_scim_base_url(input.base_url.as_deref(), false).await?;
    let row = sqlx::query_as::<_, ScimProvisioningConnectorRecord>(
        r#"
        INSERT INTO scim_provisioning_connectors (
          tenant_id,
          provider,
          status,
          base_url
        )
        VALUES ($1, $2, $3, $4)
        RETURNING
          id,
          provider,
          status,
          base_url,
          created_at
        "#,
    )
    .bind(tenant_id)
    .bind(provider)
    .bind(status)
    .bind(base_url)
    .fetch_one(db)
    .await?;

    Ok(ScimProvisioningConnectorResponse {
        connector: row.into_view(),
    })
}

pub async fn update_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    connector_id: Uuid,
    input: UpdateScimProvisioningConnectorInput,
) -> Result<ScimProvisioningConnectorResponse, AppError> {
    let current = fetch_scim_connector(db, tenant_id, connector_id).await?;
    let provider = match input.provider {
        Some(value) => normalize_scim_provider(&value)?,
        None => current.provider,
    };
    let base_url = match input.base_url {
        Some(value) => normalize_scim_base_url(Some(&value), false).await?,
        None => current.base_url,
    };
    let status = match input.status {
        Some(value) => normalize_registry_status(Some(&value))?,
        None => current.status.clone(),
    };

    let row = sqlx::query_as::<_, ScimProvisioningConnectorRecord>(
        r#"
        UPDATE scim_provisioning_connectors
        SET provider = $3,
            status = $4,
            base_url = $5
        WHERE id = $1
          AND tenant_id = $2
        RETURNING
          id,
          provider,
          status,
          base_url,
          created_at
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .bind(provider)
    .bind(status)
    .bind(base_url)
    .fetch_one(db)
    .await?;

    Ok(ScimProvisioningConnectorResponse {
        connector: row.into_view(),
    })
}

pub async fn delete_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    connector_id: Uuid,
) -> Result<(), AppError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM scim_provisioning_connectors
        WHERE id = $1
          AND tenant_id = $2
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .execute(db)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Err(AppError::not_found(
            crate::domains::federation::contract::CONNECTOR_NOT_FOUND,
            "SCIM provisioning connector not found.",
        ));
    }

    Ok(())
}

#[cfg(test)]
#[path = "identity.domains.federation.scim.tests.rs"]
mod tests;

pub async fn fetch_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    connector_id: Uuid,
) -> Result<ScimProvisioningConnectorRecord, AppError> {
    let row = sqlx::query_as::<_, ScimProvisioningConnectorRecord>(
        r#"
        SELECT
          id,
          provider,
          status,
          base_url,
          created_at
        FROM scim_provisioning_connectors
        WHERE id = $1
          AND tenant_id = $2
        LIMIT 1
        "#,
    )
    .bind(connector_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    row.ok_or_else(|| {
        AppError::not_found(
            crate::domains::federation::contract::CONNECTOR_NOT_FOUND,
            "SCIM connector not found.",
        )
    })
}
