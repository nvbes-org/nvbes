use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    clients::ClientRegistry,
    error::OAuthError,
    pkce::is_sha256_base64url,
    request::AuthorizationRequest,
    session,
    store::{StoreError, hash, random_secret},
};
use crate::browser::BrowserProof;

/// A transaction-specific CSRF token is returned only to the hosted Identity UI.
pub struct StartedInteraction {
    pub request: AuthorizationRequest,
    pub csrf_token: String,
    pub needs_login: bool,
}

#[derive(sqlx::FromRow)]
pub(super) struct StoredInteraction {
    pub parameters: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub bound_session_id: Option<Uuid>,
}

/// Called immediately after creating a direct/PAR authorization transaction.
/// A request can be attached to one browser only, and is never a login session.
pub async fn begin(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    browser_token: &str,
    session_token: Option<&str>,
) -> Result<StartedInteraction, StoreError> {
    if !is_sha256_base64url(handle) || !is_sha256_base64url(browser_token) {
        return Err(OAuthError::InvalidRequest.into());
    }
    let mut tx = db.begin().await?;
    let stored: StoredInteraction = sqlx::query_as("SELECT parameters,created_at,bound_session_id FROM identity_oauth_requests WHERE handle_hash=$1 AND kind='authorization' AND browser_hash IS NULL AND expires_at>clock_timestamp() FOR UPDATE")
        .bind(hash(handle)).fetch_optional(&mut *tx).await?.ok_or(OAuthError::InvalidRequest)?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    let session = session::load(&mut tx, session_token)
        .await?
        .filter(|s| session::is_fresh(s, &request, stored.created_at));
    let csrf_token = random_secret();
    sqlx::query("UPDATE identity_oauth_requests SET browser_hash=$2,csrf_hash=$3,bound_session_id=$4 WHERE handle_hash=$1")
        .bind(hash(handle)).bind(hash(browser_token)).bind(hash(&csrf_token)).bind(session.as_ref().map(|s| s.id))
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StartedInteraction {
        request,
        csrf_token,
        needs_login: session.is_none(),
    })
}

/// The authenticator supplies a newly issued token. Recheck its evidence and
/// rotate CSRF before showing the consent page for that exact principal/session.
pub async fn attach_authenticated_session(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
    new_session_token: &str,
) -> Result<String, StoreError> {
    let mut tx = db.begin().await?;
    let csrf = attach_session(&mut tx, clients, handle, proof, new_session_token).await?;
    tx.commit().await?;
    Ok(csrf)
}

pub(super) async fn attach_session(
    tx: &mut Transaction<'_, Postgres>,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
    new_session_token: &str,
) -> Result<String, StoreError> {
    let stored = lock(tx, handle, &proof.browser_token, Some(&proof.csrf_token)).await?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    if request.prompt.as_deref() == Some("none") {
        return Err(OAuthError::InvalidRequest.into());
    }
    let session = session::load(tx, Some(new_session_token))
        .await?
        .ok_or(OAuthError::LoginRequired)?;
    if session.authentication.authenticated_at < stored.created_at
        || !session::is_fresh(&session, &request, stored.created_at)
    {
        return Err(OAuthError::LoginRequired.into());
    }
    let csrf = random_secret();
    sqlx::query(
        "UPDATE identity_oauth_requests SET bound_session_id=$2,csrf_hash=$3 WHERE handle_hash=$1",
    )
    .bind(hash(handle))
    .bind(session.id)
    .bind(hash(&csrf))
    .execute(&mut **tx)
    .await?;
    Ok(csrf)
}

pub(super) async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    handle: &str,
    browser_token: &str,
    csrf: Option<&str>,
) -> Result<StoredInteraction, StoreError> {
    if !is_sha256_base64url(handle) || !is_sha256_base64url(browser_token) {
        return Err(OAuthError::InvalidRequest.into());
    }
    let row = sqlx::query_as("SELECT parameters,created_at,bound_session_id FROM identity_oauth_requests WHERE handle_hash=$1 AND kind='authorization' AND browser_hash=$2 AND ($3::bytea IS NULL OR csrf_hash=$3) AND expires_at>clock_timestamp() FOR UPDATE")
        .bind(hash(handle)).bind(hash(browser_token)).bind(csrf.map(hash))
        .fetch_optional(&mut **tx).await?.ok_or(OAuthError::InvalidRequest)?;
    Ok(row)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.interactions.tests.rs"]
mod tests;
