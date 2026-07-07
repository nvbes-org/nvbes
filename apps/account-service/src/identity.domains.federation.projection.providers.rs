use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::federation::{
        projection::{empty_to_option, parse_required_time, parse_required_uuid},
        types::FederatedIdentityProviderRecord,
    },
    grpc_pb::nvbes::enterprise::v1 as enterprise,
    http::error::AppError,
};

pub async fn upsert_identity_provider_projection(
    db: &PgPool,
    provider: &enterprise::FederationProvider,
) -> Result<FederatedIdentityProviderRecord, AppError> {
    let provider_id = parse_required_uuid(&provider.provider_id, "provider_id")?;
    let tenant_id = parse_required_uuid(&provider.tenant_id, "tenant_id")?;
    let created_at = parse_required_time(&provider.created_at, "created_at")?;
    let sp_entity_id = empty_to_option(&provider.sp_entity_id);
    let attribute_mapping = attribute_mapping(&provider.attribute_mapping_json)?;

    let row = sqlx::query_as::<_, FederatedIdentityProviderRecord>(
        r#"
        INSERT INTO federated_identity_providers (
          id,
          tenant_id,
          provider_type,
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
        )
        VALUES (
          $1,
          $2,
          $3::identity_provider_type,
          $4,
          $5,
          $6,
          $7,
          $8,
          $9,
          $10,
          $11,
          $12,
          $13,
          $14,
          $15
        )
        ON CONFLICT (id)
        DO UPDATE SET
          provider_type = EXCLUDED.provider_type,
          provider_family = EXCLUDED.provider_family,
          name = EXCLUDED.name,
          client_id = EXCLUDED.client_id,
          issuer = EXCLUDED.issuer,
          metadata_url = EXCLUDED.metadata_url,
          status = EXCLUDED.status,
          sp_entity_id = EXCLUDED.sp_entity_id,
          attribute_mapping = EXCLUDED.attribute_mapping,
          encryption_cert_pem = EXCLUDED.encryption_cert_pem,
          require_signed_assertions = EXCLUDED.require_signed_assertions,
          require_signed_responses = EXCLUDED.require_signed_responses,
          created_at = EXCLUDED.created_at
        WHERE federated_identity_providers.tenant_id = EXCLUDED.tenant_id
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
    .bind(&provider.protocol)
    .bind(&provider.provider_family)
    .bind(&provider.name)
    .bind(empty_to_option(&provider.client_id))
    .bind(empty_to_option(&provider.issuer))
    .bind(empty_to_option(&provider.metadata_url))
    .bind(&provider.status)
    .bind(sp_entity_id)
    .bind(attribute_mapping)
    .bind(empty_to_option(&provider.encryption_cert_pem))
    .bind(provider.require_signed_assertions)
    .bind(provider.require_signed_responses)
    .bind(created_at)
    .fetch_one(db)
    .await?;

    Ok(row)
}

pub async fn delete_identity_provider_projection(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM federated_identity_providers
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .execute(db)
    .await?;

    Ok(())
}

fn attribute_mapping(value: &str) -> Result<serde_json::Value, AppError> {
    match empty_to_option(value) {
        Some(value) => serde_json::from_str(&value).map_err(|error| {
            AppError::internal(
                "enterprise_projection_invalid",
                format!("attribute_mapping_json from Enterprise is invalid JSON: {error}"),
            )
        }),
        None => Ok(serde_json::json!({})),
    }
}
