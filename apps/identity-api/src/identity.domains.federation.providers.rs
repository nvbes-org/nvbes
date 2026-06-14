use super::types::*;
use super::validation::{
    normalize_enterprise_provider_family, normalize_federated_provider_type,
    normalize_registry_status, opt_trimmed, validate_federation_endpoint_url_allowed,
};
use crate::http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.federation.providers.validation.rs"]
mod validation;

use validation::validate_provider_configuration;

pub async fn list_identity_providers(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<FederatedIdentityProvidersResponse, AppError> {
    let rows = sqlx::query_as::<_, FederatedIdentityProviderRecord>(
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
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(FederatedIdentityProvidersResponse {
        providers: rows
            .into_iter()
            .map(FederatedIdentityProviderRecord::into_view)
            .collect(),
    })
}

pub async fn create_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
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
    let row = sqlx::query_as::<_, FederatedIdentityProviderRecord>(
        r#"
        INSERT INTO federated_identity_providers (
          tenant_id,
          provider_type,
          provider_family,
          name,
          client_id,
          issuer,
          metadata_url,
          status
        )
        VALUES ($1, $2::identity_provider_type, $3, $4, $5, $6, $7, $8)
        RETURNING
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
        "#,
    )
    .bind(tenant_id)
    .bind(provider_type)
    .bind(provider_family)
    .bind(input.name.trim())
    .bind(opt_trimmed(input.client_id))
    .bind(issuer)
    .bind(metadata_url)
    .bind(status)
    .fetch_one(db)
    .await?;

    Ok(FederatedIdentityProviderResponse {
        provider: row.into_view(),
    })
}

pub async fn update_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    input: UpdateFederatedIdentityProviderInput,
    strict_mode: bool,
) -> Result<FederatedIdentityProviderResponse, AppError> {
    let current = fetch_identity_provider(db, tenant_id, provider_id).await?;
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

    let row = sqlx::query_as::<_, FederatedIdentityProviderRecord>(
        r#"
        UPDATE federated_identity_providers
        SET provider_type = $3::identity_provider_type,
            provider_family = $4,
            name = $5,
            client_id = $6,
            issuer = $7,
            metadata_url = $8,
            status = $9
        WHERE id = $1
          AND tenant_id = $2
        RETURNING
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
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .bind(provider_type)
    .bind(provider_family)
    .bind(name)
    .bind(client_id)
    .bind(issuer)
    .bind(metadata_url)
    .bind(status)
    .fetch_one(db)
    .await?;

    Ok(FederatedIdentityProviderResponse {
        provider: row.into_view(),
    })
}

pub async fn delete_identity_provider(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM federated_identity_providers
        WHERE id = $1
          AND tenant_id = $2
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .execute(db)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Err(AppError::not_found(
            crate::domains::federation::contract::PROVIDER_NOT_FOUND,
            "Federated identity provider not found.",
        ));
    }

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
