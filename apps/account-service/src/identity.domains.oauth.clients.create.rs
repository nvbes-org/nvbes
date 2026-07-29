use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::cloud_boundary::workspace_port;
use crate::domains::oauth::{
    hash_client_secret,
    logic::OAuthManagementAuth,
    parse_client_type, parse_step_up_level,
    profiles::{
        OAuthSecurityProfile, OAuthSenderConstraint, validate_high_assurance_configuration,
    },
    service::types::{CreateOAuthClientInput, CreateOAuthClientResult, OAuthClientView},
};
use crate::http::error::AppError;
use nvbes_core::authz::parse_role;

use super::service_accounts;

/// Create a new OAuth client.
pub async fn create_client(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    input: CreateOAuthClientInput,
) -> Result<CreateOAuthClientResult, AppError> {
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let (owner_scope_type, owner_scope_id) = crate::domains::oauth::logic::resolve_owner_scope(
        auth,
        input.owner_scope_type.as_deref(),
        input.owner_scope_id,
    )?;
    let client_type = parse_client_type(input.client_type.as_deref().unwrap_or("confidential"))?;
    let client_id = format!("gxoc_{}", Uuid::new_v4().simple());
    let client_secret = format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let client_secret_hash = hash_client_secret(&client_secret)?;
    let required_acr = input.required_acr.as_deref().unwrap_or("aal1");
    let required_acr = parse_step_up_level(required_acr)?;

    if input.redirect_uris.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one redirect URI is required.",
        ));
    }
    validate_redirect_uris(&input.redirect_uris, client_type.as_str())?;
    validate_event_endpoint(
        input.backchannel_logout_uri.as_deref(),
        "backchannel_logout_uri",
    )?;
    validate_event_endpoint(
        input.security_event_receiver_uri.as_deref(),
        "security_event_receiver_uri",
    )?;

    if input.allowed_scopes.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one allowed scope is required.",
        ));
    }
    let client_assertion_required = input.client_assertion_required.unwrap_or(false);
    let requires_admin_consent = input.requires_admin_consent.unwrap_or(false);
    let security_profile = OAuthSecurityProfile::parse(input.security_profile.as_deref())?;
    let sender_constraint = OAuthSenderConstraint::parse(input.sender_constraint.as_deref())?;
    if client_assertion_required && input.client_assertion_public_key_jwk.is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "A client_assertion_public_key_jwk is required when private_key_jwt is mandatory.",
        ));
    }
    if let Some(ref jwk) = input.client_assertion_public_key_jwk
        && !crate::domains::oauth::client_assertion::is_supported_client_assertion_public_jwk(jwk)
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "client_assertion_public_key_jwk must be a supported RSA or P-256 public JWK.",
        ));
    }
    validate_high_assurance_configuration(
        security_profile,
        client_type.as_str(),
        client_assertion_required,
        input.client_assertion_public_key_jwk.as_ref(),
        input.request_object_signing_jwks.as_ref(),
        sender_constraint,
        input.tls_client_certificate_sha256.as_deref(),
    )?;
    let client_assertion_public_key_configured = input.client_assertion_public_key_jwk.is_some();
    let requested_service_account_role = input
        .service_account_role
        .as_deref()
        .unwrap_or("member")
        .trim()
        .to_lowercase();
    if parse_role(&requested_service_account_role).is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Unsupported service account workspace role.",
        ));
    }
    if requested_service_account_role == "owner" {
        let owner_access = if owner_scope_type == "workspace" {
            workspace_port::list_workspace_members(
                Some(tenant_id),
                owner_scope_id,
                OAuthManagementAuth::user_id(auth),
            )
            .await?
            .into_iter()
            .any(|member| {
                member.principal_id == OAuthManagementAuth::user_id(auth)
                    && member.active
                    && member.role == "owner"
            })
        } else {
            false
        };

        if !owner_access {
            return Err(AppError::forbidden(
                "service_account_owner_role_forbidden",
                "Only workspace owners can create owner-level service accounts.",
            ));
        }
    }

    let mut tx = db.begin().await?;
    let client_row = sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          client_id,
          client_secret_hash,
          name,
          redirect_uris,
          tenant_id,
          owner_scope_type,
          owner_scope_id,
          client_type,
          client_assertion_public_key_jwk,
          client_assertion_required,
          requires_admin_consent,
          backchannel_logout_uri,
          backchannel_logout_session_required,
          security_event_receiver_uri,
          security_profile,
          request_object_signing_jwks,
          sender_constraint,
          tls_client_certificate_sha256
        )
        VALUES (
          $1, $2, $3, $4, $5, $6::scope_type, $7, $8::client_type, $9, $10, $11,
          $12, $13, $14, $15::oauth_security_profile, $16,
          $17::oauth_sender_constraint, $18
        )
        RETURNING id, created_at
        "#,
    )
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(&input.name)
    .bind(&input.redirect_uris)
    .bind(tenant_id)
    .bind(owner_scope_type.as_str())
    .bind(owner_scope_id)
    .bind(client_type.as_str())
    .bind(input.client_assertion_public_key_jwk.clone())
    .bind(client_assertion_required)
    .bind(requires_admin_consent)
    .bind(input.backchannel_logout_uri.as_deref())
    .bind(input.backchannel_logout_session_required.unwrap_or(true))
    .bind(input.security_event_receiver_uri.as_deref())
    .bind(security_profile.as_str())
    .bind(input.request_object_signing_jwks.clone())
    .bind(sender_constraint.map(OAuthSenderConstraint::as_str))
    .bind(input.tls_client_certificate_sha256.as_deref())
    .fetch_one(&mut *tx)
    .await?;
    let client_uuid: Uuid = client_row.get("id");
    let client_created_at: chrono::DateTime<chrono::Utc> = client_row.get("created_at");
    let service_account_principal_id = if client_type == "service" {
        let workspace_id = require_workspace_scope(&owner_scope_type, owner_scope_id)?;
        Some(
            service_accounts::ensure_service_account_for_client(
                &mut tx,
                auth,
                &client_id,
                workspace_id,
                &input,
                &requested_service_account_role,
            )
            .await?,
        )
    } else {
        None
    };
    super::keys::insert_initial_keys(
        &mut tx,
        tenant_id,
        &client_id,
        input.client_assertion_public_key_jwk.as_ref(),
        input.request_object_signing_jwks.as_ref(),
        security_profile == OAuthSecurityProfile::HighAssurance,
    )
    .await?;

    let policy = sqlx::query(
        r#"
        INSERT INTO oauth_client_policies (
          client_id,
          scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr,
          status
        )
        VALUES ($1, $2::scope_type, $3, $4, $5, $6, $7::step_up_level, 'active')
        RETURNING id, scope_type::text AS scope_type, scope_id, allowed_scopes, allowed_audiences, allowed_resources, required_acr::text AS required_acr, status::text AS status, created_at
        "#,
    )
    .bind(client_uuid)
    .bind(owner_scope_type.as_str())
    .bind(owner_scope_id)
    .bind(&input.allowed_scopes)
    .bind(&input.allowed_audiences)
    .bind(&input.allowed_resources)
    .bind(required_acr.as_str())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let is_service_client = client_type == "service";

    Ok(CreateOAuthClientResult {
        client: OAuthClientView {
            id: client_uuid,
            client_id,
            name: input.name,
            redirect_uris: input.redirect_uris,
            created_at: client_created_at,
            last_used_at: None,
            tenant_id: Some(tenant_id),
            owner_scope_type,
            owner_scope_id,
            client_type: client_type.clone(),
            client_assertion_required,
            requires_admin_consent,
            client_assertion_public_key_configured,
            security_profile: security_profile.as_str().to_string(),
            request_object_signing_keys_configured: input.request_object_signing_jwks.is_some(),
            sender_constraint: sender_constraint
                .map(OAuthSenderConstraint::as_str)
                .map(str::to_string),
            tls_client_certificate_bound_access_tokens: sender_constraint
                == Some(OAuthSenderConstraint::Mtls),
            backchannel_logout_uri: input.backchannel_logout_uri,
            backchannel_logout_session_required: input
                .backchannel_logout_session_required
                .unwrap_or(true),
            security_event_receiver_uri: input.security_event_receiver_uri,
            service_account_principal_id,
            service_account_workspace_id: is_service_client.then_some(owner_scope_id),
            service_account_role: is_service_client
                .then_some(requested_service_account_role.clone()),
        },
        client_secret,
        policy: super::map_client_policy_row_with_client_id(&policy, client_uuid),
    })
}

fn validate_event_endpoint(value: Option<&str>, field: &str) -> Result<(), AppError> {
    let Some(value) = value else {
        return Ok(());
    };
    let parsed = url::Url::parse(value).map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            format!("{field} must be an absolute URL."),
        )
    })?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || parsed.fragment().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} must be an HTTPS URL without credentials or fragment."),
        ));
    }
    if let Some(url::Host::Ipv4(address)) = parsed.host()
        && (address.is_private()
            || address.is_loopback()
            || address.is_link_local()
            || address.is_unspecified())
    {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} cannot target a private network."),
        ));
    }
    if let Some(url::Host::Ipv6(address)) = parsed.host()
        && (address.is_loopback() || address.is_unspecified())
    {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} cannot target a private network."),
        ));
    }
    Ok(())
}

fn validate_redirect_uris(values: &[String], client_type: &str) -> Result<(), AppError> {
    if values.len() > 20 {
        return Err(AppError::bad_request(
            "validation_failed",
            "At most 20 redirect URIs may be registered.",
        ));
    }
    let mut unique = std::collections::HashSet::with_capacity(values.len());
    for value in values {
        let parsed = url::Url::parse(value).map_err(|_| {
            AppError::bad_request(
                "validation_failed",
                "Every redirect URI must be an absolute URL.",
            )
        })?;
        let loopback_http = crate::domains::oauth::validation::is_public_client_type(client_type)
            && parsed.scheme() == "http"
            && parsed
                .host_str()
                .is_some_and(|host| matches!(host, "127.0.0.1" | "::1" | "localhost"));
        if (parsed.scheme() != "https" && !loopback_http)
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.host_str().is_none()
            || value.contains('*')
            || !unique.insert(value)
        {
            return Err(AppError::bad_request(
                "validation_failed",
                "Redirect URIs must be unique exact HTTPS URLs; loopback HTTP is allowed only for native clients.",
            ));
        }
    }
    Ok(())
}

fn require_workspace_scope(owner_scope_type: &str, owner_scope_id: Uuid) -> Result<Uuid, AppError> {
    if owner_scope_type != "workspace" {
        return Err(AppError::bad_request(
            "workspace_scope_required",
            "Service OAuth clients must be scoped to a workspace.",
        ));
    }

    Ok(owner_scope_id)
}
