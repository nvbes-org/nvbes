use crate::domains::auth::sessions::cache::{cached_session_from_login, current_session_ttl};
use chrono::Utc;
use uuid::Uuid;

use super::principal::{first_workspace_context, resolve_or_create_principal};
use crate::{
    domains::{
        auth::jwt::JwtService,
        federation::{
            identities::link_or_reuse_identity,
            types::{
                FederatedIdentityProviderRecord, InboundFederationInput, InboundFederationResponse,
                JitProvisioningInput, JitProvisioningResponse,
            },
            validation::normalize_federated_provider_type,
        },
    },
    http::error::AppError,
};
use nvbes_core::auth::{normalize_email, validate_email};
use sqlx::PgPool;

pub async fn jit_provision(
    db: &PgPool,
    tenant_id: Uuid,
    input: JitProvisioningInput,
) -> Result<JitProvisioningResponse, AppError> {
    let email = normalize_email(&input.email);
    validate_email(&email)?;
    let domain = email
        .split_once('@')
        .map(|(_, domain)| domain.to_string())
        .ok_or_else(|| {
            AppError::bad_request("validation_failed", "The email address is invalid.")
        })?;

    let verified_domain = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_domains
        WHERE tenant_id = $1
          AND lower(domain) = $2
          AND verified_at IS NOT NULL
        "#,
    )
    .bind(tenant_id)
    .bind(&domain)
    .fetch_one(db)
    .await?;

    if verified_domain == 0 {
        return Err(AppError::forbidden(
            crate::domains::federation::contract::DOMAIN_NOT_VERIFIED,
            "The email domain is not verified for this tenant.",
        ));
    }

    let provider_type = normalize_federated_provider_type(&input.provider_type)?;

    let (principal_id, created_account, created_membership) = resolve_or_create_principal(
        db,
        tenant_id,
        &email,
        input.username.trim(),
        input.email_verified,
    )
    .await?;

    let identity = link_or_reuse_identity(
        db,
        principal_id,
        &provider_type,
        Uuid::parse_str(&input.provider_id).map_err(|_| {
            AppError::bad_request(
                crate::domains::federation::contract::INVALID_PROVIDER_ID,
                "Invalid provider ID",
            )
        })?,
        &email,
        input.subject.trim(),
        input.email_verified,
    )
    .await?;

    Ok(JitProvisioningResponse {
        principal_id,
        user_id: principal_id,
        tenant_id,
        linked_identity_id: identity.id,
        created_account,
        created_membership,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "Inbound federation keeps provider, session, and protocol context explicit."
)]
pub async fn handle_inbound_federation(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth_session_ttl_hours: i64,
    tenant_id: Uuid,
    provider: FederatedIdentityProviderRecord,
    input: InboundFederationInput,
    protocol: &str,
) -> Result<InboundFederationResponse, AppError> {
    if provider.status != "active" {
        return Err(AppError::forbidden(
            crate::domains::federation::contract::PROVIDER_INACTIVE,
            "The identity provider is not active.",
        ));
    }

    let email = normalize_email(&input.email);
    validate_email(&email)?;
    let provider_type = provider.provider_type.clone();

    let (principal_id, created_account, created_membership) =
        resolve_or_create_principal(db, tenant_id, &email, &input.username, input.email_verified)
            .await?;

    let linked_identity = link_or_reuse_identity(
        db,
        principal_id,
        &provider_type,
        input.provider_id,
        &email,
        &input.subject,
        input.email_verified,
    )
    .await?;

    let (workspace_id, organization_id, workspace_region) =
        first_workspace_context(db, principal_id).await?;
    let session_id = Uuid::new_v4();
    let scope = "openid profile email offline_access".to_string();
    let amr = vec!["federated".to_string(), protocol.to_string()];
    let token_pair = jwt.generate_token_pair_with_session(
        principal_id,
        workspace_id,
        workspace_region.clone(),
        &scope,
        Some(session_id),
        Some(tenant_id),
        organization_id,
        Some("aal1"),
        Some(amr.clone()),
        Some(&provider.name),
        Some(Utc::now().timestamp()),
        None,
    )?;

    let now = Utc::now();
    let cached_session = cached_session_from_login(
        session_id,
        principal_id,
        Some(tenant_id),
        organization_id,
        workspace_id,
        workspace_region.clone(),
        Some(provider.name.clone()),
        nvbes_core::auth::token_hash(&token_pair.access_token),
        Some("aal1".to_string()),
        amr.clone(),
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(auth_session_ttl_hours),
    );
    nvbes_redis::session::set_session(redis, &cached_session, current_session_ttl(&cached_session))
        .await
        .map_err(|err| AppError::internal("session_cache_write_failed", err.to_string()))?;

    Ok(InboundFederationResponse {
        principal_id,
        user_id: principal_id,
        tenant_id,
        workspace_id,
        session_id,
        token_type: token_pair.token_type,
        expires_in: token_pair.expires_in,
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        scope,
        provider_type,
        provider_name: provider.name,
        linked_identity_id: linked_identity.id,
        created_account,
        created_membership,
    })
}
