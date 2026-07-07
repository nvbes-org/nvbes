use super::types::*;
use super::validation::{
    normalize_enterprise_provider_family, normalize_federated_provider_type,
    normalize_registry_status, opt_trimmed, validate_federation_endpoint_url_allowed,
};
use crate::{
    domains::{
        enterprise::grpc::federation::{
            self as enterprise_federation, ConfigureFederationProviderCommand,
        },
        federation::provider_projection,
    },
    http::error::AppError,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.federation.providers.validation.rs"]
mod validation;

use validation::validate_provider_configuration;

pub async fn list_identity_providers(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<FederatedIdentityProvidersResponse, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    Ok(FederatedIdentityProvidersResponse {
        providers: governance
            .providers
            .into_iter()
            .map(provider_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

pub async fn create_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: CreateFederatedIdentityProviderInput,
    strict_mode: bool,
) -> Result<FederatedIdentityProviderResponse, AppError> {
    let provider_type = normalize_federated_provider_type(&input.provider_type)?;
    let provider_family = normalize_enterprise_provider_family(input.provider_family.as_deref())?;
    validate_provider_configuration(
        &provider_type,
        &provider_family,
        input.client_id.as_deref(),
        input.issuer.as_deref(),
        input.metadata_url.as_deref(),
    )?;
    let status = normalize_registry_status(input.status.as_deref())?;
    let issuer = match input.issuer {
        Some(ref value) => {
            Some(validate_federation_endpoint_url_allowed(value, strict_mode).await?)
        }
        None => None,
    };
    let metadata_url = match input.metadata_url {
        Some(ref value) => {
            Some(validate_federation_endpoint_url_allowed(value, strict_mode).await?)
        }
        None => None,
    };
    let provider = enterprise_federation::configure_federation_provider(
        tenant_id,
        actor_principal_id,
        ConfigureFederationProviderCommand {
            provider_id: None,
            provider_type,
            provider_family,
            name: input.name.trim().to_string(),
            client_id: opt_trimmed(input.client_id),
            issuer,
            metadata_url,
            status,
            sp_entity_id: None,
            attribute_mapping_json: serde_json::json!({}).to_string(),
            encryption_cert_pem: None,
            require_signed_assertions: true,
            require_signed_responses: true,
        },
    )
    .await?;
    let row = provider_projection::upsert_identity_provider_projection(db, &provider).await?;

    Ok(FederatedIdentityProviderResponse {
        provider: row.into_view(),
    })
}

pub async fn update_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    provider_id: Uuid,
    input: UpdateFederatedIdentityProviderInput,
    strict_mode: bool,
) -> Result<FederatedIdentityProviderResponse, AppError> {
    let current =
        fetch_governance_identity_provider(tenant_id, actor_principal_id, provider_id).await?;
    let provider_type = match input.provider_type {
        Some(value) => normalize_federated_provider_type(&value)?,
        None => current.provider_type.clone(),
    };
    let provider_family = match input.provider_family {
        Some(value) => normalize_enterprise_provider_family(Some(&value))?,
        None => current.provider_family.clone(),
    };
    let name = input.name.unwrap_or(current.name).trim().to_string();
    let client_id = input.client_id.or(current.client_id);
    let issuer = match input.issuer.or(current.issuer) {
        Some(value) => Some(validate_federation_endpoint_url_allowed(&value, strict_mode).await?),
        None => None,
    };
    let metadata_url = match input.metadata_url.or(current.metadata_url) {
        Some(value) => Some(validate_federation_endpoint_url_allowed(&value, strict_mode).await?),
        None => None,
    };
    let status = match input.status {
        Some(value) => normalize_registry_status(Some(&value))?,
        None => current.status.clone(),
    };
    validate_provider_configuration(
        &provider_type,
        &provider_family,
        client_id.as_deref(),
        issuer.as_deref(),
        metadata_url.as_deref(),
    )?;

    let provider = enterprise_federation::configure_federation_provider(
        tenant_id,
        actor_principal_id,
        ConfigureFederationProviderCommand {
            provider_id: Some(provider_id),
            provider_type,
            provider_family,
            name,
            client_id,
            issuer,
            metadata_url,
            status,
            sp_entity_id: current.sp_entity_id,
            attribute_mapping_json: current.attribute_mapping.to_string(),
            encryption_cert_pem: current.encryption_cert_pem,
            require_signed_assertions: current.require_signed_assertions,
            require_signed_responses: current.require_signed_responses,
        },
    )
    .await?;
    let row = provider_projection::upsert_identity_provider_projection(db, &provider).await?;

    Ok(FederatedIdentityProviderResponse {
        provider: row.into_view(),
    })
}

pub async fn delete_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    fetch_governance_identity_provider(tenant_id, actor_principal_id, provider_id).await?;
    enterprise_federation::delete_federation_provider(tenant_id, actor_principal_id, provider_id)
        .await?;
    provider_projection::delete_identity_provider_projection(db, tenant_id, provider_id).await?;
    Ok(())
}

pub async fn fetch_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
) -> Result<FederatedIdentityProviderRecord, AppError> {
    let row = sqlx::query_as::<_, FederatedIdentityProviderRecord>(
        r#"
        SELECT
          id,
          provider_type::text AS provider_type,
          provider_family,
          name,
          client_id,
          issuer,
          metadata_url,
          status,
          sp_entity_id,
          attribute_mapping,
          encryption_cert_pem,
          require_signed_assertions,
          require_signed_responses,
          created_at
        FROM federated_identity_providers
        WHERE id = $1
          AND tenant_id = $2
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    row.ok_or_else(|| {
        AppError::not_found(
            crate::domains::federation::contract::PROVIDER_NOT_FOUND,
            "Federated identity provider not found.",
        )
    })
}

async fn fetch_governance_identity_provider(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    provider_id: Uuid,
) -> Result<FederatedIdentityProviderView, AppError> {
    let governance =
        enterprise_federation::get_federation_governance(tenant_id, actor_principal_id).await?;
    governance
        .providers
        .into_iter()
        .find(|provider| provider.provider_id == provider_id.to_string())
        .map(provider_from_grpc)
        .transpose()?
        .ok_or_else(|| {
            AppError::not_found(
                crate::domains::federation::contract::PROVIDER_NOT_FOUND,
                "Federated identity provider not found.",
            )
        })
}

fn provider_from_grpc(
    provider: crate::grpc_pb::nvbes::enterprise::v1::FederationProvider,
) -> Result<FederatedIdentityProviderView, AppError> {
    Ok(FederatedIdentityProviderView {
        id: parse_uuid(&provider.provider_id, "provider_id")?,
        provider_type: provider.protocol,
        provider_family: provider.provider_family,
        name: provider.name,
        client_id: optional_text(provider.client_id),
        issuer: optional_text(provider.issuer),
        metadata_url: optional_text(provider.metadata_url),
        status: provider.status,
        sp_entity_id: optional_text(provider.sp_entity_id),
        attribute_mapping: parse_json(&provider.attribute_mapping_json)?,
        encryption_cert_pem: optional_text(provider.encryption_cert_pem),
        require_signed_assertions: provider.require_signed_assertions,
        require_signed_responses: provider.require_signed_responses,
        created_at: parse_time(&provider.created_at, "created_at")?,
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

fn parse_time(value: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_federation_governance",
                format!("Enterprise gRPC returned invalid {field}: {error}"),
            )
        })
}

fn parse_json(value: &str) -> Result<serde_json::Value, AppError> {
    if value.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(value).map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_federation_governance",
                format!("Enterprise gRPC returned invalid provider attribute mapping: {error}"),
            )
        })
    }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
