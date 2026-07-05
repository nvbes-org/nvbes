use super::types::*;
use super::validation::normalize_domain;
use crate::http::error::AppError;
use chrono::Utc;
use nvbes_core::auth::{generate_token, token_hash};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_tenant_domains(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<TenantDomainsResponse, AppError> {
    let rows = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        SELECT id, domain, sso_required, sso_provider_id, verified_at,
               verification_requested_at, verification_expires_at,
               verification_token_hash, created_at
        FROM tenant_domains
        WHERE tenant_id = $1
        ORDER BY domain ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(TenantDomainsResponse {
        domains: rows
            .into_iter()
            .map(TenantDomainRecord::into_view)
            .collect(),
    })
}

pub async fn create_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    input: CreateTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let domain = normalize_domain(&input.domain)?;
    if input.sso_required.unwrap_or(false) {
        validate_sso_provider(db, tenant_id, input.sso_provider_id).await?;
    }

    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_domains
        WHERE tenant_id = $1
          AND lower(domain) = $2
        "#,
    )
    .bind(tenant_id)
    .bind(&domain)
    .fetch_one(db)
    .await?;

    if existing > 0 {
        return Err(AppError::conflict(
            "domain_already_registered",
            "This domain is already registered for the tenant.",
        ));
    }

    let verification_token = generate_token("dom");
    let token_hash = token_hash(&verification_token);
    let row = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        INSERT INTO tenant_domains (
          tenant_id, domain, sso_required, sso_provider_id, verified_at,
          verification_requested_at, verification_expires_at, verification_token_hash
        )
        VALUES ($1, $2, $3, $4, NULL, NOW(), NOW() + INTERVAL '24 hours', $5)
        RETURNING id, domain, sso_required, sso_provider_id, verified_at,
                  verification_requested_at, verification_expires_at,
                  verification_token_hash, created_at
        "#,
    )
    .bind(tenant_id)
    .bind(&domain)
    .bind(input.sso_required.unwrap_or(false))
    .bind(input.sso_provider_id)
    .bind(&token_hash)
    .fetch_one(db)
    .await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: Some(verification_token),
    })
}

pub async fn update_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
    input: UpdateTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let current = fetch_tenant_domain_for_update(db, tenant_id, domain_id).await?;
    let sso_required = input.sso_required.unwrap_or(current.sso_required);
    let sso_provider_id = input.sso_provider_id.or(current.sso_provider_id);

    if sso_required {
        validate_sso_provider(db, tenant_id, sso_provider_id).await?;
    }

    let row = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        UPDATE tenant_domains
        SET sso_required = $3,
            sso_provider_id = CASE WHEN $3 THEN $4 ELSE NULL END
        WHERE id = $1
          AND tenant_id = $2
        RETURNING id, domain, sso_required, sso_provider_id, verified_at,
                  verification_requested_at, verification_expires_at,
                  verification_token_hash, created_at
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .bind(sso_required)
    .bind(sso_provider_id)
    .fetch_one(db)
    .await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: None,
    })
}

pub async fn verify_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
    input: VerifyTenantDomainInput,
) -> Result<TenantDomainResponse, AppError> {
    let row = fetch_tenant_domain_for_update(db, tenant_id, domain_id).await?;
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

    let row = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        UPDATE tenant_domains
        SET verified_at = NOW(),
            verification_requested_at = NULL,
            verification_expires_at = NULL,
            verification_token_hash = NULL
        WHERE id = $1
          AND tenant_id = $2
        RETURNING id, domain, sso_required, sso_provider_id, verified_at,
                  verification_requested_at, verification_expires_at,
                  verification_token_hash, created_at
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    Ok(TenantDomainResponse {
        domain: row.into_view(),
        verification_token: None,
    })
}

pub async fn delete_tenant_domain(
    db: &PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
) -> Result<(), AppError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM tenant_domains
        WHERE id = $1
          AND tenant_id = $2
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .execute(db)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Err(AppError::not_found(
            crate::domains::federation::contract::DOMAIN_NOT_FOUND,
            "Domain not found.",
        ));
    }

    Ok(())
}

async fn fetch_tenant_domain_for_update(
    db: &PgPool,
    tenant_id: Uuid,
    domain_id: Uuid,
) -> Result<TenantDomainRecord, AppError> {
    let row = sqlx::query_as::<_, TenantDomainRecord>(
        r#"
        SELECT id, domain, sso_required, sso_provider_id, verified_at,
               verification_requested_at, verification_expires_at,
               verification_token_hash, created_at
        FROM tenant_domains
        WHERE id = $1
          AND tenant_id = $2
        FOR UPDATE
        "#,
    )
    .bind(domain_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    row.ok_or_else(|| {
        AppError::not_found(
            crate::domains::federation::contract::DOMAIN_NOT_FOUND,
            "Domain not found.",
        )
    })
}

async fn validate_sso_provider(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Option<Uuid>,
) -> Result<(), AppError> {
    let provider_id = provider_id.ok_or_else(|| {
        AppError::bad_request(
            "sso_provider_required",
            "A domain with required SSO must reference an active OIDC or SAML provider.",
        )
    })?;

    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM federated_identity_providers
          WHERE id = $1
            AND tenant_id = $2
            AND provider_type IN ('oidc', 'saml')
            AND status = 'active'
        )
        "#,
    )
    .bind(provider_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    if !exists {
        return Err(AppError::bad_request(
            "sso_provider_invalid",
            "The required SSO provider must be an active OIDC or SAML provider in this tenant.",
        ));
    }

    Ok(())
}
