use sqlx::{PgPool, Postgres, Transaction};

use super::{
    clients::ClientRegistry,
    codes::{self, AuthorizationCode},
    error::OAuthError,
    interactions,
    request::AuthorizationRequest,
    session::{self, ActiveSession},
    store::{StoreError, audit, hash},
};
use crate::browser::BrowserProof;

pub async fn approve(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
) -> Result<AuthorizationCode, StoreError> {
    let mut tx = db.begin().await?;
    let stored = interactions::lock(
        &mut tx,
        handle,
        &proof.browser_token,
        Some(&proof.csrf_token),
    )
    .await?;
    let request = AuthorizationRequest::restore(stored.parameters.clone(), clients)?;
    if request.prompt.as_deref() == Some("none") {
        return Err(OAuthError::InvalidRequest.into());
    }
    let session = authenticated(&mut tx, &request, &stored, proof.session_token.as_deref()).await?;
    sqlx::query("INSERT INTO identity_oauth_consents(principal_id,client_id,policy_hash,resource,audience,scope) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(principal_id,client_id,policy_hash) DO UPDATE SET granted_at=clock_timestamp(),expires_at=clock_timestamp()+interval '30 days'")
        .bind(session.principal_id).bind(&request.client_id).bind(policy_hash(&request))
        .bind(&request.resource).bind(&request.audience).bind(&request.scope).execute(&mut *tx).await?;
    audit(
        &mut tx,
        session.principal_id,
        "identity.oauth.consent_granted",
    )
    .await?;
    let code = finish(&mut tx, handle, request, &session).await?;
    tx.commit().await?;
    Ok(code)
}

/// No UI is allowed for prompt=none. A prior exact consent is mandatory;
/// offline access always requires an explicit interaction in this deployment.
pub async fn silent(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    browser_token: &str,
    session_token: Option<&str>,
) -> Result<AuthorizationCode, StoreError> {
    let mut tx = db.begin().await?;
    let stored = interactions::lock(&mut tx, handle, browser_token, None).await?;
    let request = AuthorizationRequest::restore(stored.parameters.clone(), clients)?;
    if request.prompt.as_deref() != Some("none") {
        return Err(OAuthError::InvalidRequest.into());
    }
    let session = authenticated(&mut tx, &request, &stored, session_token).await?;
    let granted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_oauth_consents WHERE principal_id=$1 AND client_id=$2 AND policy_hash=$3 AND expires_at>clock_timestamp())")
        .bind(session.principal_id).bind(&request.client_id).bind(policy_hash(&request)).fetch_one(&mut *tx).await?;
    if !granted || request.scope.split(' ').any(|s| s == "offline_access") {
        return Err(OAuthError::ConsentRequired.into());
    }
    let code = finish(&mut tx, handle, request, &session).await?;
    tx.commit().await?;
    Ok(code)
}

pub async fn deny(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
) -> Result<AuthorizationRequest, StoreError> {
    let mut tx = db.begin().await?;
    let stored = interactions::lock(
        &mut tx,
        handle,
        &proof.browser_token,
        Some(&proof.csrf_token),
    )
    .await?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    // Refusal is valid even after the login cookie has expired. No grant is made.
    let principal: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
            .bind(stored.bound_session_id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(principal) = principal {
        audit(&mut tx, principal, "identity.oauth.consent_denied").await?;
    }
    consume(&mut tx, handle).await?;
    tx.commit().await?;
    Ok(request)
}

async fn authenticated(
    tx: &mut Transaction<'_, Postgres>,
    request: &AuthorizationRequest,
    stored: &interactions::StoredInteraction,
    token: Option<&str>,
) -> Result<ActiveSession, StoreError> {
    let session = session::load(tx, token)
        .await?
        .ok_or(OAuthError::LoginRequired)?;
    if stored.bound_session_id != Some(session.id)
        || !session::is_fresh(&session, request, stored.created_at)
    {
        return Err(OAuthError::LoginRequired.into());
    }
    Ok(session)
}

async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    handle: &str,
    request: AuthorizationRequest,
    session: &ActiveSession,
) -> Result<AuthorizationCode, StoreError> {
    consume(tx, handle).await?;
    codes::issue_code(tx, request, session).await
}

async fn consume(tx: &mut Transaction<'_, Postgres>, handle: &str) -> Result<(), StoreError> {
    let count = sqlx::query(
        "DELETE FROM identity_oauth_requests WHERE handle_hash=$1 AND expires_at>clock_timestamp()",
    )
    .bind(hash(handle))
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if count != 1 {
        return Err(OAuthError::InvalidRequest.into());
    }
    Ok(())
}

fn policy_hash(request: &AuthorizationRequest) -> Vec<u8> {
    hash(&serde_json::json!([request.resource, request.audience, request.scope]).to_string())
}
