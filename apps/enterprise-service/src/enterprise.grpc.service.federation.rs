use chrono::Utc;
use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::{
    federation,
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, validate_context},
};

pub(super) async fn configure_federation_provider(
    db: &PgPool,
    request: Request<enterprise::ConfigureFederationProviderRequest>,
) -> Result<Response<enterprise::FederationProvider>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let provider = federation::configure_federation_provider(db, tenant_id, request).await?;
    Ok(Response::new(provider))
}

pub(super) async fn delete_federation_provider(
    db: &PgPool,
    request: Request<enterprise::DeleteFederationProviderRequest>,
) -> Result<Response<enterprise::FederationProviderDeletion>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let provider_id = parse_uuid(&request.provider_id, "provider_id")?;
    Ok(Response::new(
        federation::delete_federation_provider(db, tenant_id, provider_id).await?,
    ))
}

pub(super) async fn configure_tenant_domain(
    db: &PgPool,
    request: Request<enterprise::ConfigureTenantDomainRequest>,
) -> Result<Response<enterprise::TenantDomain>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        federation::configure_tenant_domain(db, tenant_id, request).await?,
    ))
}

pub(super) async fn verify_tenant_domain(
    db: &PgPool,
    request: Request<enterprise::VerifyTenantDomainRequest>,
) -> Result<Response<enterprise::TenantDomain>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let domain_id = parse_uuid(&request.domain_id, "domain_id")?;
    Ok(Response::new(
        federation::verify_tenant_domain(db, tenant_id, domain_id, &request.dns_txt_token).await?,
    ))
}

pub(super) async fn delete_tenant_domain(
    db: &PgPool,
    request: Request<enterprise::DeleteTenantDomainRequest>,
) -> Result<Response<enterprise::TenantDomainDeletion>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let domain_id = parse_uuid(&request.domain_id, "domain_id")?;
    Ok(Response::new(
        federation::delete_tenant_domain(db, tenant_id, domain_id).await?,
    ))
}

pub(super) async fn configure_scim_connector(
    db: &PgPool,
    request: Request<enterprise::ConfigureScimConnectorRequest>,
) -> Result<Response<enterprise::ScimConnector>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        federation::configure_scim_connector(db, tenant_id, request).await?,
    ))
}

pub(super) async fn delete_scim_connector(
    db: &PgPool,
    request: Request<enterprise::DeleteScimConnectorRequest>,
) -> Result<Response<enterprise::ScimConnectorDeletion>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let connector_id = parse_uuid(&request.connector_id, "connector_id")?;
    Ok(Response::new(
        federation::delete_scim_connector(db, tenant_id, connector_id).await?,
    ))
}

pub(super) async fn test_federation_provider(
    request: Request<enterprise::TestFederationProviderRequest>,
) -> Result<Response<enterprise::FederationTestResult>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    Ok(Response::new(enterprise::FederationTestResult {
        provider_id: request.provider_id,
        status: "not_checked".to_string(),
        checked_at: Utc::now().to_rfc3339(),
        findings: vec!["provider test execution is not implemented".to_string()],
    }))
}

pub(super) async fn get_federation_governance(
    db: &PgPool,
    request: Request<enterprise::GetFederationGovernanceRequest>,
) -> Result<Response<enterprise::FederationGovernance>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        federation::federation_governance(db, tenant_id).await?,
    ))
}
