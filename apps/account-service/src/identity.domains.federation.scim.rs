use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        enterprise::grpc::federation::{
            self as enterprise_federation, ConfigureScimConnectorCommand,
        },
        federation::{
            contract::{normalize_scim_base_url, normalize_scim_provider},
            scim_projection,
            types::{
                CreateScimProvisioningConnectorInput, ScimProvisioningConnectorResponse,
                ScimProvisioningConnectorView, ScimProvisioningConnectorsResponse,
                UpdateScimProvisioningConnectorInput,
            },
            validation::normalize_registry_status,
        },
    },
    http::error::AppError,
};

pub async fn list_scim_connectors(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<ScimProvisioningConnectorsResponse, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    Ok(ScimProvisioningConnectorsResponse {
        connectors: governance
            .scim_connectors
            .into_iter()
            .map(connector_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

pub async fn create_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: CreateScimProvisioningConnectorInput,
) -> Result<ScimProvisioningConnectorResponse, AppError> {
    let provider = normalize_scim_provider(&input.provider)?;
    let status = normalize_registry_status(input.status.as_deref())?;
    let base_url = normalize_scim_base_url(input.base_url.as_deref(), false).await?;
    let connector = enterprise_federation::configure_scim_connector(
        tenant_id,
        actor_principal_id,
        ConfigureScimConnectorCommand {
            connector_id: None,
            provider,
            status,
            base_url,
        },
    )
    .await?;
    let row = scim_projection::upsert_scim_connector_projection(db, &connector).await?;

    Ok(ScimProvisioningConnectorResponse {
        connector: row.into_view(),
    })
}

pub async fn update_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    connector_id: Uuid,
    input: UpdateScimProvisioningConnectorInput,
) -> Result<ScimProvisioningConnectorResponse, AppError> {
    let current =
        fetch_governance_scim_connector(tenant_id, actor_principal_id, connector_id).await?;
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

    let connector = enterprise_federation::configure_scim_connector(
        tenant_id,
        actor_principal_id,
        ConfigureScimConnectorCommand {
            connector_id: Some(connector_id),
            provider,
            status,
            base_url,
        },
    )
    .await?;
    let row = scim_projection::upsert_scim_connector_projection(db, &connector).await?;

    Ok(ScimProvisioningConnectorResponse {
        connector: row.into_view(),
    })
}

pub async fn delete_scim_connector(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    connector_id: Uuid,
) -> Result<(), AppError> {
    fetch_governance_scim_connector(tenant_id, actor_principal_id, connector_id).await?;
    enterprise_federation::delete_scim_connector(tenant_id, actor_principal_id, connector_id)
        .await?;
    scim_projection::delete_scim_connector_projection(db, tenant_id, connector_id).await?;
    Ok(())
}

async fn fetch_governance_scim_connector(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    connector_id: Uuid,
) -> Result<ScimProvisioningConnectorView, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    governance
        .scim_connectors
        .into_iter()
        .find(|connector| connector.connector_id == connector_id.to_string())
        .map(connector_from_grpc)
        .transpose()?
        .ok_or_else(|| {
            AppError::not_found(
                crate::domains::federation::contract::CONNECTOR_NOT_FOUND,
                "SCIM connector not found.",
            )
        })
}

fn connector_from_grpc(
    connector: crate::grpc_pb::nvbes::enterprise::v1::ScimConnector,
) -> Result<ScimProvisioningConnectorView, AppError> {
    Ok(ScimProvisioningConnectorView {
        id: parse_uuid(&connector.connector_id, "connector_id")?,
        provider: connector.provider,
        status: connector.status,
        base_url: optional_text(connector.base_url),
        created_at: parse_time(&connector.created_at, "created_at")?,
    })
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_federation_governance",
            format!("Enterprise gRPC returned invalid {field}: {error}"),
        )
    })
}

fn parse_time(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_federation_governance",
                format!("Enterprise gRPC returned invalid {field}: {error}"),
            )
        })
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
