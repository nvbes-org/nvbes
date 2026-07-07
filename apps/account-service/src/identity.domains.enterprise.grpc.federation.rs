use uuid::Uuid;

use crate::{
    grpc_pb::nvbes::enterprise::v1::{
        self as enterprise, ConfigureFederationProviderRequest, ConfigureScimConnectorRequest,
        ConfigureTenantDomainRequest, DeleteFederationProviderRequest, DeleteScimConnectorRequest,
        DeleteTenantDomainRequest, GetFederationGovernanceRequest, VerifyTenantDomainRequest,
    },
    http::error::AppError,
};

pub struct ConfigureFederationProviderCommand {
    pub provider_id: Option<Uuid>,
    pub provider_type: String,
    pub provider_family: String,
    pub name: String,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub metadata_url: Option<String>,
    pub status: String,
    pub sp_entity_id: Option<String>,
    pub attribute_mapping_json: String,
    pub encryption_cert_pem: Option<String>,
    pub require_signed_assertions: bool,
    pub require_signed_responses: bool,
}

pub struct ConfigureTenantDomainCommand {
    pub domain_id: Option<Uuid>,
    pub domain: String,
    pub sso_required: bool,
    pub sso_provider_id: Option<Uuid>,
    pub verification_token_hash: Option<String>,
}

pub struct ConfigureScimConnectorCommand {
    pub connector_id: Option<Uuid>,
    pub provider: String,
    pub status: String,
    pub base_url: Option<String>,
}

pub async fn configure_federation_provider(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    command: ConfigureFederationProviderCommand,
) -> Result<enterprise::FederationProvider, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .configure_federation_provider(ConfigureFederationProviderRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            provider_id: option_uuid(command.provider_id),
            protocol: command.provider_type,
            issuer: option_string(command.issuer),
            metadata_url: option_string(command.metadata_url),
            scim_base_url: String::new(),
            status: command.status,
            provider_family: command.provider_family,
            name: command.name,
            client_id: option_string(command.client_id),
            sp_entity_id: option_string(command.sp_entity_id),
            attribute_mapping_json: command.attribute_mapping_json,
            encryption_cert_pem: option_string(command.encryption_cert_pem),
            require_signed_assertions: command.require_signed_assertions,
            require_signed_responses: command.require_signed_responses,
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn delete_federation_provider(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .delete_federation_provider(DeleteFederationProviderRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            provider_id: provider_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn configure_tenant_domain(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    command: ConfigureTenantDomainCommand,
) -> Result<enterprise::TenantDomain, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .configure_tenant_domain(ConfigureTenantDomainRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            domain_id: option_uuid(command.domain_id),
            domain: command.domain,
            sso_required: command.sso_required,
            sso_provider_id: option_uuid(command.sso_provider_id),
            verification_token_hash: option_string(command.verification_token_hash),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn verify_tenant_domain(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
) -> Result<enterprise::TenantDomain, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .verify_tenant_domain(VerifyTenantDomainRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            domain_id: domain_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn delete_tenant_domain(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .delete_tenant_domain(DeleteTenantDomainRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            domain_id: domain_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn configure_scim_connector(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    command: ConfigureScimConnectorCommand,
) -> Result<enterprise::ScimConnector, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .configure_scim_connector(ConfigureScimConnectorRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            connector_id: option_uuid(command.connector_id),
            provider: command.provider,
            status: command.status,
            base_url: option_string(command.base_url),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn delete_scim_connector(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    connector_id: Uuid,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .delete_scim_connector(DeleteScimConnectorRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            connector_id: connector_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn get_federation_governance(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<enterprise::FederationGovernance, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .get_federation_governance(GetFederationGovernanceRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

fn option_uuid(value: Option<Uuid>) -> String {
    value.map(|id| id.to_string()).unwrap_or_default()
}

fn option_string(value: Option<String>) -> String {
    value.unwrap_or_default()
}
