use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    federation::{optional_time_string, optional_uuid_string, time_string},
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, optional_uuid, sql_status},
};

pub async fn configure_tenant_domain(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: enterprise::ConfigureTenantDomainRequest,
) -> Result<enterprise::TenantDomain, Status> {
    let domain_id = optional_uuid(&request.domain_id, "domain_id")?.unwrap_or_else(Uuid::new_v4);
    let domain = non_empty(request.domain, "domain")?;
    let sso_provider_id = optional_uuid(&request.sso_provider_id, "sso_provider_id")?;

    let row = sqlx::query(
        r#"
        INSERT INTO tenant_domains (
          id,
          tenant_id,
          domain,
          sso_required,
          sso_provider_id,
          verification_token_hash,
          verification_requested_at,
          verification_expires_at
        )
        VALUES (
          $1,
          $2,
          $3,
          $4,
          $5,
          NULLIF($6, ''),
          CASE WHEN NULLIF($6, '') IS NULL THEN NULL ELSE NOW() END,
          CASE WHEN NULLIF($6, '') IS NULL THEN NULL ELSE NOW() + INTERVAL '24 hours' END
        )
        ON CONFLICT (id)
        DO UPDATE SET
          domain = EXCLUDED.domain,
          sso_required = EXCLUDED.sso_required,
          sso_provider_id = CASE
            WHEN EXCLUDED.sso_required THEN EXCLUDED.sso_provider_id
            ELSE NULL
          END,
          verification_token_hash = COALESCE(
            EXCLUDED.verification_token_hash,
            tenant_domains.verification_token_hash
          ),
          verification_requested_at = COALESCE(
            EXCLUDED.verification_requested_at,
            tenant_domains.verification_requested_at
          ),
          verification_expires_at = COALESCE(
            EXCLUDED.verification_expires_at,
            tenant_domains.verification_expires_at
          )
        WHERE tenant_domains.tenant_id = EXCLUDED.tenant_id
        RETURNING
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
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .bind(domain)
    .bind(request.sso_required)
    .bind(sso_provider_id)
    .bind(request.verification_token_hash.trim())
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise tenant domain was not found"))?;

    Ok(domain_from_row(row))
}

pub async fn verify_tenant_domain(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
) -> Result<enterprise::TenantDomain, Status> {
    let row = sqlx::query(
        r#"
        UPDATE tenant_domains
        SET verified_at = NOW(),
            verification_requested_at = NULL,
            verification_expires_at = NULL,
            verification_token_hash = NULL
        WHERE id = $1 AND tenant_id = $2
        RETURNING
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
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(domain_from_row(row))
}

pub async fn delete_tenant_domain(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
) -> Result<enterprise::TenantDomainDeletion, Status> {
    let row = sqlx::query(
        r#"
        DELETE FROM tenant_domains
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::TenantDomainDeletion {
        domain_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
    })
}

pub(crate) fn domain_from_row(row: PgRow) -> enterprise::TenantDomain {
    enterprise::TenantDomain {
        domain_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        domain: row.get("domain"),
        sso_required: row.get("sso_required"),
        sso_provider_id: optional_uuid_string(row.get("sso_provider_id")),
        verified_at: optional_time_string(row.get("verified_at")),
        verification_requested_at: optional_time_string(row.get("verification_requested_at")),
        verification_expires_at: optional_time_string(row.get("verification_expires_at")),
        verification_token_hash: row
            .get::<Option<String>, _>("verification_token_hash")
            .unwrap_or_default(),
        created_at: time_string(row.get("created_at")),
    }
}
