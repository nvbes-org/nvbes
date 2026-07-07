use super::types::*;
use super::validation::normalize_domain;
use crate::{
    domains::{
        enterprise::grpc::federation::{
            self as enterprise_federation, ConfigureTenantDomainCommand,
        },
        federation::domain_projection,
    },
    http::error::AppError,
};
use chrono::Utc;
use nvbes_core::auth::{generate_token, token_hash};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.federation.domains.governance.rs"]
mod governance;

pub async fn list_tenant_domains(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<TenantDomainsResponse, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    Ok(TenantDomainsResponse {
        domains: governance
            .domains
            .into_iter()
            .map(governance::tenant_domain_from_grpc)
            .map(|domain| domain.map(governance::TenantDomain::into_view))
            .collect::<Result<_, _>>()?,
    })
}

pub async fn create_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: CreateTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let domain = normalize_domain(&input.domain)?;
    if input.sso_required.unwrap_or(false) {
        validate_sso_provider(tenant_id, actor_principal_id, input.sso_provider_id).await?;
    }

    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    if governance
        .domains
        .iter()
        .any(|record| record.domain.eq_ignore_ascii_case(&domain))
    {
        return Err(AppError::conflict(
            "domain_already_registered",
            "This domain is already registered for the tenant.",
        ));
    }

    let verification_token = generate_token("dom");
    let token_hash = token_hash(&verification_token);
    let domain_record = enterprise_federation::configure_tenant_domain(
        tenant_id,
        actor_principal_id,
        ConfigureTenantDomainCommand {
            domain_id: None,
            domain,
            sso_required: input.sso_required.unwrap_or(false),
            sso_provider_id: input.sso_provider_id,
            verification_token_hash: Some(token_hash),
        },
    )
    .await?;
    let row = domain_projection::upsert_tenant_domain_projection(db, &domain_record).await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: Some(verification_token),
    })
}

pub async fn update_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
    input: UpdateTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let current = fetch_governance_tenant_domain(tenant_id, actor_principal_id, domain_id).await?;
    let sso_required = input.sso_required.unwrap_or(current.sso_required);
    let sso_provider_id = if sso_required {
        input.sso_provider_id.or(current.sso_provider_id)
    } else {
        None
    };

    if sso_required {
        validate_sso_provider(tenant_id, actor_principal_id, sso_provider_id).await?;
    }

    let domain_record = enterprise_federation::configure_tenant_domain(
        tenant_id,
        actor_principal_id,
        ConfigureTenantDomainCommand {
            domain_id: Some(domain_id),
            domain: current.domain,
            sso_required,
            sso_provider_id,
            verification_token_hash: None,
        },
    )
    .await?;
    let row = domain_projection::upsert_tenant_domain_projection(db, &domain_record).await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: None,
    })
}

pub async fn verify_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
    input: VerifyTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let row = fetch_governance_tenant_domain(tenant_id, actor_principal_id, domain_id).await?;
    let verification_token_hash = row.verification_token_hash.clone().ok_or_else(|| {
        AppError::bad_request(
            "verification_not_requested",
            "Request a verification token before verifying the domain.",
        )
    })?;

    if row
        .verification_expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(AppError::bad_request(
            "verification_expired",
            "The domain verification token has expired.",
        ));
    }

    if token_hash(input.token.trim()) != verification_token_hash {
        return Err(AppError::bad_request(
            "invalid_verification_token",
            "The domain verification token is invalid.",
        ));
    }

    let domain_record =
        enterprise_federation::verify_tenant_domain(tenant_id, actor_principal_id, domain_id)
            .await?;
    let row = domain_projection::upsert_tenant_domain_projection(db, &domain_record).await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: None,
    })
}

pub async fn delete_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
) -> Result<(), AppError> {
    fetch_governance_tenant_domain(tenant_id, actor_principal_id, domain_id).await?;
    enterprise_federation::delete_tenant_domain(tenant_id, actor_principal_id, domain_id).await?;
    domain_projection::delete_tenant_domain_projection(db, tenant_id, domain_id).await?;
    Ok(())
}

async fn fetch_governance_tenant_domain(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    domain_id: Uuid,
) -> Result<governance::TenantDomain, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    governance
        .domains
        .into_iter()
        .find(|domain| domain.domain_id == domain_id.to_string())
        .map(governance::tenant_domain_from_grpc)
        .transpose()?
        .ok_or_else(|| {
            AppError::not_found(
                crate::domains::federation::contract::DOMAIN_NOT_FOUND,
                "Domain not found.",
            )
        })
}

async fn validate_sso_provider(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    provider_id: Option<Uuid>,
) -> Result<(), AppError> {
    let provider_id = provider_id.ok_or_else(|| {
        AppError::bad_request(
            "sso_provider_required",
            "A domain with required SSO must reference an active OIDC or SAML provider.",
        )
    })?;

    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    let valid_provider = governance.providers.iter().any(|provider| {
        provider.provider_id == provider_id.to_string()
            && matches!(provider.protocol.as_str(), "oidc" | "saml")
            && provider.status == "active"
    });
    if !valid_provider {
        return Err(AppError::bad_request(
            "sso_provider_invalid",
            "The required SSO provider must be an active OIDC or SAML provider in this tenant.",
        ));
    }

    Ok(())
}
