use sha2::{Digest, Sha256};
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
    let domain = normalize_domain(&non_empty(request.domain, "domain")?)?;
    let sso_provider_id = optional_uuid(&request.sso_provider_id, "sso_provider_id")?;
    if request.sso_required {
        let provider_id = sso_provider_id.ok_or_else(|| {
            Status::invalid_argument("sso_provider_id is required when SSO is enforced")
        })?;
        ensure_active_provider(db, tenant_id, provider_id).await?;
    }

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
          verified_at = CASE
            WHEN lower(tenant_domains.domain) = lower(EXCLUDED.domain)
              THEN tenant_domains.verified_at
            ELSE NULL
          END,
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
    dns_txt_token: &str,
) -> Result<enterprise::TenantDomain, Status> {
    let dns_txt_token = non_empty(dns_txt_token.to_string(), "dns_txt_token")?;
    let token_hash = format!("{:x}", Sha256::digest(dns_txt_token.as_bytes()));
    let row = sqlx::query(
        r#"
        UPDATE tenant_domains
        SET verified_at = NOW(),
            verification_requested_at = NULL,
            verification_expires_at = NULL,
            verification_token_hash = NULL
        WHERE id = $1
          AND tenant_id = $2
          AND verification_token_hash IS NOT NULL
          AND verification_token_hash = $3
          AND verification_expires_at > NOW()
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
    .bind(token_hash)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition("domain verification challenge is missing or expired")
    })?;

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
        verification_token_hash: String::new(),
        created_at: time_string(row.get("created_at")),
    }
}

async fn ensure_active_provider(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
) -> Result<(), Status> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM federated_identity_providers
          WHERE id = $1 AND tenant_id = $2 AND status = 'active'
        )
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;
    if exists {
        Ok(())
    } else {
        Err(Status::failed_precondition(
            "SSO provider must be active and belong to the tenant",
        ))
    }
}

fn normalize_domain(value: &str) -> Result<String, Status> {
    let domain = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.len() > 253
        || domain.parse::<std::net::IpAddr>().is_ok()
        || domain.contains('*')
        || domain.contains('@')
    {
        return Err(Status::invalid_argument("domain is not a valid DNS name"));
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(Status::invalid_argument("domain is not a valid DNS name"));
    }
    Ok(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_normalization_rejects_takeover_prone_inputs() {
        assert_eq!(
            normalize_domain("Example.COM.").expect("domain should normalize"),
            "example.com"
        );
        assert!(normalize_domain("*.example.com").is_err());
        assert!(normalize_domain("127.0.0.1").is_err());
        assert!(normalize_domain("single-label").is_err());
    }
}
