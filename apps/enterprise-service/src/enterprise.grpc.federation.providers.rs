use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    federation::{empty_to_option, time_string},
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{optional_uuid, sql_status},
};

pub async fn configure_federation_provider(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: enterprise::ConfigureFederationProviderRequest,
) -> Result<enterprise::FederationProvider, Status> {
    let provider_id =
        optional_uuid(&request.provider_id, "provider_id")?.unwrap_or_else(Uuid::new_v4);
    let provider_family = if request.provider_family.trim().is_empty() {
        "custom".to_string()
    } else {
        request.provider_family.trim().to_string()
    };
    let attribute_mapping = attribute_mapping(&request.attribute_mapping_json)?;

    let row = sqlx::query(
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
          require_signed_responses
        )
        VALUES (
          $1,
          $2,
          $3::identity_provider_type,
          $4,
          $5,
          NULLIF($6, ''),
          NULLIF($7, ''),
          NULLIF($8, ''),
          $9,
          NULLIF($10, ''),
          $11,
          NULLIF($12, ''),
          $13,
          $14
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
          require_signed_responses = EXCLUDED.require_signed_responses
        WHERE federated_identity_providers.tenant_id = EXCLUDED.tenant_id
        RETURNING
          id,
          tenant_id,
          provider_type::text AS protocol,
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
    .bind(request.protocol.trim())
    .bind(provider_family)
    .bind(request.name.trim())
    .bind(request.client_id.trim())
    .bind(request.issuer.trim())
    .bind(request.metadata_url.trim())
    .bind(default_status(&request.status))
    .bind(request.sp_entity_id.trim())
    .bind(attribute_mapping)
    .bind(request.encryption_cert_pem.trim())
    .bind(request.require_signed_assertions)
    .bind(request.require_signed_responses)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise federation provider was not found"))?;

    Ok(provider_from_row(row, request.scim_base_url))
}

pub async fn delete_federation_provider(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
) -> Result<enterprise::FederationProviderDeletion, Status> {
    let row = sqlx::query(
        r#"
        DELETE FROM federated_identity_providers
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::FederationProviderDeletion {
        provider_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
    })
}

pub(crate) fn provider_from_row(
    row: PgRow,
    scim_base_url: String,
) -> enterprise::FederationProvider {
    enterprise::FederationProvider {
        provider_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        protocol: row.get("protocol"),
        issuer: row.get::<Option<String>, _>("issuer").unwrap_or_default(),
        metadata_url: row
            .get::<Option<String>, _>("metadata_url")
            .unwrap_or_default(),
        scim_base_url,
        status: row.get("status"),
        provider_family: row.get("provider_family"),
        name: row.get("name"),
        client_id: row
            .get::<Option<String>, _>("client_id")
            .unwrap_or_default(),
        sp_entity_id: row
            .get::<Option<String>, _>("sp_entity_id")
            .unwrap_or_default(),
        attribute_mapping_json: row
            .get::<serde_json::Value, _>("attribute_mapping")
            .to_string(),
        encryption_cert_pem: row
            .get::<Option<String>, _>("encryption_cert_pem")
            .unwrap_or_default(),
        require_signed_assertions: row.get("require_signed_assertions"),
        require_signed_responses: row.get("require_signed_responses"),
        created_at: time_string(row.get("created_at")),
    }
}

fn attribute_mapping(value: &str) -> Result<serde_json::Value, Status> {
    Ok(match empty_to_option(value) {
        Some(value) => serde_json::from_str(&value)
            .map_err(|_| Status::invalid_argument("attribute_mapping_json must be JSON"))?,
        None => serde_json::json!({}),
    })
}

fn default_status(value: &str) -> String {
    empty_to_option(value).unwrap_or_else(|| "active".to_string())
}
