use super::{
    clients::ClientRegistry,
    error::OAuthError,
    interactions,
    login::validate_interaction,
    store::{self, StoreError, hash, random_secret},
};
use crate::browser::BrowserProof;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::{Webauthn, prelude::*};

#[derive(serde::Serialize)]
pub struct LoginOptions {
    pub ceremony_id: Uuid,
    pub options: RequestChallengeResponse,
}

pub struct AuthenticatedLogin {
    pub token: String,
    pub csrf: String,
}

/// The HTTP adapter must verify Origin and enforce source quotas before calling.
/// BrowserProof and the OAuth interaction provide browser/CSRF continuity.
pub async fn start(
    db: &PgPool,
    server: &Webauthn,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
) -> Result<LoginOptions, StoreError> {
    let mut tx = db.begin().await?;
    validate_interaction(&mut tx, clients, handle, proof).await?;
    let (options, state) = server
        .start_discoverable_authentication()
        .map_err(|_| OAuthError::InvalidRequest)?;
    let id = Uuid::new_v4();
    sqlx::query("DELETE FROM identity_webauthn_challenges WHERE oauth_request_hash=$1 AND purpose='authentication'")
        .bind(hash(handle)).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO identity_webauthn_challenges(id,challenge,purpose,expires_at,ceremony_state,oauth_request_hash) VALUES($1,$2,'authentication',clock_timestamp()+interval '5 minutes',$3,$4)")
        .bind(id).bind(options.public_key.challenge.as_ref()).bind(serde_json::to_value(state)?).bind(hash(handle)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(LoginOptions {
        ceremony_id: id,
        options,
    })
}

pub async fn finish(
    db: &PgPool,
    server: &Webauthn,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
    ceremony: Uuid,
    response: &PublicKeyCredential,
) -> Result<AuthenticatedLogin, StoreError> {
    let mut tx = db.begin().await?;
    validate_interaction(&mut tx, clients, handle, proof).await?;
    let state: Option<serde_json::Value> = sqlx::query_scalar("SELECT ceremony_state FROM identity_webauthn_challenges WHERE id=$1 AND oauth_request_hash=$2 AND purpose='authentication' AND consumed_at IS NULL AND expires_at>clock_timestamp() AND ceremony_state IS NOT NULL FOR UPDATE")
        .bind(ceremony).bind(hash(handle)).fetch_optional(&mut *tx).await?;
    let state: DiscoverableAuthentication =
        serde_json::from_value(state.ok_or(OAuthError::InvalidRequest)?)?;
    // userHandle is an untrusted lookup hint. Only the matching credential's
    // verified signature below establishes the principal's identity.
    let (principal, credential_id) = server
        .identify_discoverable_authentication(response)
        .map_err(|_| OAuthError::InvalidRequest)?;
    let active: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM identity_principals WHERE id=$1 AND status='active' FOR UPDATE",
    )
    .bind(principal)
    .fetch_optional(&mut *tx)
    .await?;
    active.ok_or(OAuthError::InvalidRequest)?;
    let row: Option<(Uuid, serde_json::Value)> = sqlx::query_as("SELECT id,passkey FROM identity_webauthn_credentials WHERE principal_id=$1 AND credential_id=$2 AND revoked_at IS NULL AND passkey IS NOT NULL FOR UPDATE")
        .bind(principal).bind(credential_id).fetch_optional(&mut *tx).await?;
    let (id, value) = row.ok_or(OAuthError::InvalidRequest)?;
    let mut passkey: Passkey = serde_json::from_value(value)?;
    let discoverable: DiscoverableKey = passkey.clone().into();
    let verified = server
        .finish_discoverable_authentication(response, state, &[discoverable])
        .map_err(|_| OAuthError::InvalidRequest)?;
    if !verified.user_verified() || passkey.update_credential(&verified).is_none() {
        return Err(OAuthError::InvalidRequest.into());
    }
    sqlx::query("UPDATE identity_webauthn_credentials SET passkey=$1,sign_count=$2,backed_up=$3,last_used_at=clock_timestamp() WHERE id=$4")
        .bind(serde_json::to_value(passkey)?).bind(i64::from(verified.counter())).bind(verified.backup_state()).bind(id).execute(&mut *tx).await?;
    let token = random_secret();
    let session = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_sessions(id,principal_id,token_hash,expires_at,authenticated_at,primary_amr) VALUES($1,$2,$3,clock_timestamp()+interval '1 hour',clock_timestamp(),'webauthn')")
        .bind(session).bind(principal).bind(hash(&token)).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO identity_session_webauthn_credentials(session_id,credential_id,principal_id,purpose) VALUES($1,$2,$3,'primary')")
        .bind(session).bind(id).bind(principal).execute(&mut *tx).await?;
    sqlx::query("UPDATE identity_webauthn_challenges SET consumed_at=clock_timestamp(),principal_id=$2 WHERE id=$1")
        .bind(ceremony).bind(principal).execute(&mut *tx).await?;
    store::audit(&mut tx, principal, "identity.session.passkey_authenticated").await?;
    let csrf = interactions::attach_session(&mut tx, clients, handle, proof, &token).await?;
    tx.commit().await?;
    Ok(AuthenticatedLogin { token, csrf })
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.passkey_login.tests.rs"]
mod tests;
