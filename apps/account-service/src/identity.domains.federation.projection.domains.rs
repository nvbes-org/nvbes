use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::federation::{
        projection::{
            empty_to_option, parse_optional_time, parse_optional_uuid, parse_required_time,
            parse_required_uuid,
        },
        types::TenantDomainRecord,
    },
    grpc_pb::nvbes::enterprise::v1 as enterprise,
    http::error::AppError,
};

pub async fn upsert_tenant_domain_projection(
    db: &PgPool,
    domain: &enterprise::TenantDomain,
) -> Result<TenantDomainRecord, AppError> {
    let domain_id = parse_required_uuid(&domain.domain_id, "domain_id")?;
    let tenant_id = parse_required_uuid(&domain.tenant_id, "tenant_id")?;
    let sso_provider_id = parse_optional_uuid(&domain.sso_provider_id, "sso_provider_id")?;
    let verified_at = parse_optional_time(&domain.verified_at, "verified_at")?;
    let verification_requested_at = parse_optional_time(
        &domain.verification_requested_at,
        "verification_requested_at",
    )?;
    let verification_expires_at =
        parse_optional_time(&domain.verification_expires_at, "verification_expires_at")?;
    let created_at = parse_required_time(&domain.created_at, "created_at")?;

    let row = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        INSERT INTO tenant_domains (
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
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (id)
        DO UPDATE SET
          domain = EXCLUDED.domain,
          sso_required = EXCLUDED.sso_required,
          sso_provider_id = EXCLUDED.sso_provider_id,
          verified_at = EXCLUDED.verified_at,
          verification_requested_at = EXCLUDED.verification_requested_at,
          verification_expires_at = EXCLUDED.verification_expires_at,
          verification_token_hash = EXCLUDED.verification_token_hash,
          created_at = EXCLUDED.created_at
        WHERE tenant_domains.tenant_id = EXCLUDED.tenant_id
        RETURNING
          id,
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
    .bind(&domain.domain)
    .bind(domain.sso_required)
    .bind(sso_provider_id)
    .bind(verified_at)
    .bind(verification_requested_at)
    .bind(verification_expires_at)
    .bind(empty_to_option(&domain.verification_token_hash))
    .bind(created_at)
    .fetch_one(db)
    .await?;

    Ok(row)
}

pub async fn delete_tenant_domain_projection(
    db: &PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM tenant_domains
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .execute(db)
    .await?;

    Ok(())
}
