use super::{
    clients::ClientRegistry,
    error::OAuthError,
    interactions,
    request::AuthorizationRequest,
    store::{StoreError, hash},
};
use crate::{auth, browser::BrowserProof};
use sqlx::{PgPool, Postgres, Transaction};

pub(super) struct LoginSession {
    pub token: String,
    pub csrf: String,
}

pub(super) async fn authenticate(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
    email: &str,
    password: &str,
) -> Result<LoginSession, StoreError> {
    // Reject expired, replayed or foreign interactions before password work,
    // without holding a database transaction during Argon2 verification.
    let mut preflight = db.begin().await?;
    validate_interaction(&mut preflight, clients, handle, proof).await?;
    preflight.commit().await?;
    let verified = auth::verify_credentials(db, email, password)
        .await
        .map_err(authentication_error)?;
    let mut tx = db.begin().await?;
    validate_interaction(&mut tx, clients, handle, proof).await?;
    let token = auth::create_verified_session(&mut tx, verified)
        .await
        .map_err(authentication_error)?;
    let csrf = interactions::attach_session(&mut tx, clients, handle, proof, &token).await?;
    tx.commit().await?;
    Ok(LoginSession { token, csrf })
}

async fn validate_interaction(
    tx: &mut Transaction<'_, Postgres>,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
) -> Result<(), StoreError> {
    let stored =
        interactions::lock(tx, handle, &proof.browser_token, Some(&proof.csrf_token)).await?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    if request.prompt.as_deref() == Some("none") {
        return Err(OAuthError::InvalidRequest.into());
    }
    if let Some(bound) = stored.bound_session_id {
        // Browser continuity is not authentication. Fresh credentials authorize
        // the new session even if the old one expired. Do not lock its principal.
        let token = proof
            .session_token
            .as_deref()
            .ok_or(OAuthError::InvalidRequest)?;
        let current: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM identity_sessions WHERE token_hash=$1")
                .bind(hash(token))
                .fetch_optional(&mut **tx)
                .await?;
        if current != Some(bound) {
            return Err(OAuthError::InvalidRequest.into());
        }
    }
    Ok(())
}

fn authentication_error(error: anyhow::Error) -> StoreError {
    if error
        .downcast_ref::<sqlx::Error>()
        .is_some_and(|error| !matches!(error, sqlx::Error::RowNotFound))
    {
        OAuthError::Unavailable.into()
    } else {
        OAuthError::InvalidRequest.into()
    }
}
