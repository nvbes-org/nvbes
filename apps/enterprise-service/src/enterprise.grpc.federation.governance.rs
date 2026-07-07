use tonic::Status;
use uuid::Uuid;

use crate::grpc::{pb::nvbes::enterprise::v1 as enterprise, service_status::sql_status};

use super::{domains::domain_from_row, providers::provider_from_row, scim::connector_from_row};

pub async fn federation_governance(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::FederationGovernance, Status> {
    let providers = sqlx::query(
        r#"
        SELECT
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
        FROM federated_identity_providers
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(|row| provider_from_row(row, String::new()))
    .collect();

    let domains = sqlx::query(
        r#"
        SELECT
          id,
          tenant_id,
          domain,
          sso_required,
          sso_provider_id,
          verified_at,
          verification_requested_at,
          verification_expires_at,
          verification_token_hash,
          created_at
        FROM tenant_domains
        WHERE tenant_id = $1
        ORDER BY domain ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(domain_from_row)
    .collect();

    let scim_connectors: Vec<_> = sqlx::query(
        r#"
        SELECT id, tenant_id, provider, status, base_url, created_at
        FROM scim_provisioning_connectors
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(connector_from_row)
    .collect();

    let scim_required = scim_connectors
        .iter()
        .any(|connector| connector.status == "active");

    Ok(enterprise::FederationGovernance {
        tenant_id: tenant_id.to_string(),
        providers,
        required_claims: Vec::new(),
        scim_required,
        domains,
        scim_connectors,
    })
}
