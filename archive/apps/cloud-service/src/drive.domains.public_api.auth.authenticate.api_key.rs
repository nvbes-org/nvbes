use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

use super::super::super::{
    api_key_signatures, db,
    errors::PublicApiErrorKind,
    http_signatures, observability,
    types::{DeniedLogInput, PublicApiContext},
};
use super::PublicApiCredential;

#[expect(
    clippy::too_many_arguments,
    reason = "Public API authentication keeps request and principal context explicit."
)]
pub(super) async fn authenticate_api_key(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &axum::http::HeaderMap,
    method: &axum::http::Method,
    uri: &axum::http::Uri,
    request_id: String,
    required_scope: &str,
    credential: PublicApiCredential,
    row: sqlx::postgres::PgRow,
) -> Result<PublicApiContext, AppError> {
    let workspace_id: Uuid = row.get("workspace_id");
    let api_key_id: Uuid = row.get("id");
    let status: String = row.get("status");
    let scopes: Vec<String> = row.get("scopes");
    let plan_code: String = row.get("plan_code");
    let created_by_principal_id: Uuid = row.get("created_by_principal_id");
    let deleted_at: Option<chrono::DateTime<Utc>> = row.get("deleted_at");
    let expires_at: Option<chrono::DateTime<Utc>> = row.get("expires_at");
    let public_key: Option<String> = row.get("http_signature_public_key");

    match credential {
        PublicApiCredential::HttpSignature(_) => {
            let public_key = public_key
                .as_deref()
                .ok_or_else(|| PublicApiErrorKind::HttpSignatureNotEnabled.app_error())?;
            http_signatures::verify(headers, method, uri, public_key)?;
        }
        PublicApiCredential::Bearer(_) if public_key.is_some() => {
            return Err(PublicApiErrorKind::HttpSignatureRequired.app_error());
        }
        PublicApiCredential::Bearer(token) => {
            let verified = api_key_signatures::verify(headers, method, uri, &token)?;
            consume_nonce(db, workspace_id, api_key_id, verified).await?;
        }
    }

    ensure_api_key_allowed(
        db,
        headers,
        request_id.as_str(),
        required_scope,
        workspace_id,
        api_key_id,
        created_by_principal_id,
        &status,
        &scopes,
        deleted_at,
        expires_at,
    )
    .await?;
    enforce_network_policy(
        db,
        headers,
        request_id.as_str(),
        required_scope,
        workspace_id,
        Some(api_key_id),
        Some(created_by_principal_id),
    )
    .await?;

    super::enforce_plan_rate_limit(redis, "api_key_rate", &api_key_id.to_string(), &plan_code)
        .await?;

    db::update_last_used(
        db,
        api_key_id,
        crate::http::request::client_ip(headers).as_deref(),
    )
    .await?;

    Ok(PublicApiContext {
        api_key_id: Some(api_key_id),
        workspace_id,
        created_by: Some(row.get("created_by")),
        created_by_principal_id: row.get("created_by_principal_id"),
        tenant_id: None,
        organization_id: None,
        role: None,
        key_prefix: row.get("key_prefix"),
        scopes,
        plan_code,
        request_id,
        m2m_client_id: None,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "Denied audit logging needs explicit request metadata and actor context."
)]
pub(super) async fn enforce_network_policy(
    db: &PgPool,
    headers: &axum::http::HeaderMap,
    request_id: &str,
    required_scope: &str,
    workspace_id: Uuid,
    api_key_id: Option<Uuid>,
    actor_principal_id: Option<Uuid>,
) -> Result<(), AppError> {
    let ip = crate::http::request::client_ip(headers);
    let Some(block) = crate::domains::public_api::network_policy::public_api_network_block(
        db,
        workspace_id,
        ip.as_deref(),
    )
    .await?
    else {
        return Ok(());
    };

    crate::domains::public_api::metrics::record_network_policy_block(
        block.reason,
        block.mode.as_str(),
    );
    if !matches!(
        block.mode,
        crate::domains::public_api::network_policy::PublicApiNetworkPolicyMode::Enforce
    ) {
        return Ok(());
    }

    observability::log_denied(
        db,
        DeniedLogInput {
            workspace_id,
            api_key_id,
            actor_principal_id,
            request_id,
            error_code: PublicApiErrorKind::NetworkRiskBlocked.code(),
            network_block_reason: Some(block.reason),
            ip: ip.as_deref(),
            user_agent: crate::http::request::user_agent(headers).as_deref(),
            scopes_used: &[required_scope],
        },
    )
    .await?;
    Err(PublicApiErrorKind::NetworkRiskBlocked.app_error())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Denied audit paths keep explicit request context for logs."
)]
async fn ensure_api_key_allowed(
    db: &PgPool,
    headers: &axum::http::HeaderMap,
    request_id: &str,
    required_scope: &str,
    workspace_id: Uuid,
    api_key_id: Uuid,
    created_by_principal_id: Uuid,
    status: &str,
    scopes: &[String],
    deleted_at: Option<chrono::DateTime<Utc>>,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> Result<(), AppError> {
    if status == "revoked" {
        return denied(
            db,
            headers,
            request_id,
            required_scope,
            workspace_id,
            api_key_id,
            created_by_principal_id,
            PublicApiErrorKind::RevokedApiKey,
        )
        .await;
    }
    if status == "expired" || expires_at.is_some_and(|value| value <= Utc::now()) {
        return denied(
            db,
            headers,
            request_id,
            required_scope,
            workspace_id,
            api_key_id,
            created_by_principal_id,
            PublicApiErrorKind::ExpiredApiKey,
        )
        .await;
    }
    if deleted_at.is_some() {
        return denied(
            db,
            headers,
            request_id,
            required_scope,
            workspace_id,
            api_key_id,
            created_by_principal_id,
            PublicApiErrorKind::WorkspaceInactive,
        )
        .await;
    }
    if !scopes.iter().any(|scope| scope == required_scope) {
        return denied(
            db,
            headers,
            request_id,
            required_scope,
            workspace_id,
            api_key_id,
            created_by_principal_id,
            PublicApiErrorKind::InsufficientApiKeyScope,
        )
        .await;
    }

    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Denied audit logging needs explicit request metadata and actor context."
)]
async fn denied(
    db: &PgPool,
    headers: &axum::http::HeaderMap,
    request_id: &str,
    required_scope: &str,
    workspace_id: Uuid,
    api_key_id: Uuid,
    created_by_principal_id: Uuid,
    kind: PublicApiErrorKind,
) -> Result<(), AppError> {
    observability::log_denied(
        db,
        DeniedLogInput {
            workspace_id,
            api_key_id: Some(api_key_id),
            actor_principal_id: Some(created_by_principal_id),
            request_id,
            error_code: kind.code(),
            network_block_reason: None,
            ip: crate::http::request::client_ip(headers).as_deref(),
            user_agent: crate::http::request::user_agent(headers).as_deref(),
            scopes_used: &[required_scope],
        },
    )
    .await?;
    Err(kind.app_error())
}

async fn consume_nonce(
    db: &PgPool,
    workspace_id: Uuid,
    api_key_id: Uuid,
    verified: api_key_signatures::VerifiedApiKeySignature,
) -> Result<(), AppError> {
    if db::consume_api_key_nonce(
        db,
        workspace_id,
        api_key_id,
        &verified.nonce,
        verified.timestamp,
    )
    .await?
    {
        return Ok(());
    }
    Err(PublicApiErrorKind::ApiKeyNonceReplayed.app_error())
}
