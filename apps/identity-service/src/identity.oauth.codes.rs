use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    clients::ClientRegistry,
    error::OAuthError,
    pkce,
    request::AuthorizationRequest,
    store::{StoreError, audit, hash, random_secret},
};

pub struct AuthorizationCode {
    pub code: String,
    pub request: AuthorizationRequest,
}

pub struct AuthorizedGrant {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub request: AuthorizationRequest,
}

/// Called only after hosted authentication and consent. Session authentication
/// itself is rechecked under a row lock; the caller cannot supply a principal.
pub async fn authorize(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    session_token: &str,
) -> Result<AuthorizationCode, StoreError> {
    let mut tx = db.begin().await?;
    let (session_id, principal_id, authenticated_at): (Uuid, Uuid, DateTime<Utc>) =
        sqlx::query_as("SELECT s.id,s.principal_id,s.created_at FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p")
            .bind(hash(session_token)).fetch_optional(&mut *tx).await?
            .ok_or(OAuthError::AccessDenied)?;
    let value: serde_json::Value = sqlx::query_scalar("DELETE FROM identity_oauth_requests WHERE handle_hash=$1 AND kind='authorization' AND expires_at>clock_timestamp() RETURNING parameters")
        .bind(hash(handle)).fetch_optional(&mut *tx).await?.ok_or(OAuthError::InvalidRequest)?;
    let request = AuthorizationRequest::restore(value.clone(), clients)?;
    if request
        .max_age
        .is_some_and(|max| (Utc::now() - authenticated_at).num_seconds() > i64::from(max))
    {
        return Err(OAuthError::AccessDenied.into());
    }
    let code = random_secret();
    sqlx::query("INSERT INTO identity_oauth_codes(code_hash,session_id,principal_id,client_id,parameters) VALUES($1,$2,$3,$4,$5)")
        .bind(hash(&code)).bind(session_id).bind(principal_id).bind(&request.client_id).bind(value)
        .execute(&mut *tx).await?;
    audit(&mut tx, principal_id, "identity.oauth.authorized").await?;
    tx.commit().await?;
    Ok(AuthorizationCode { code, request })
}

pub struct CodeExchange<'a> {
    pub code: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub verifier: &'a str,
    /// Thumbprint from a verified DPoP proof, never an unverified header field.
    pub verified_dpop_jkt: Option<&'a str>,
}

#[derive(sqlx::FromRow)]
struct StoredCode {
    principal_id: Uuid,
    session_id: Uuid,
    parameters: serde_json::Value,
    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
}

pub async fn exchange(
    db: &PgPool,
    clients: &ClientRegistry,
    input: CodeExchange<'_>,
) -> Result<AuthorizedGrant, StoreError> {
    let mut tx = db.begin().await?;
    let code: StoredCode = sqlx::query_as("SELECT principal_id,session_id,parameters,consumed_at,expires_at FROM identity_oauth_codes WHERE code_hash=$1 AND client_id=$2 FOR UPDATE")
        .bind(hash(input.code)).bind(input.client_id).fetch_optional(&mut *tx).await?
        .ok_or(OAuthError::InvalidGrant)?;
    let request = AuthorizationRequest::restore(code.parameters.clone(), clients)?;
    if request.redirect_uri != input.redirect_uri
        || request.dpop_jkt.as_deref() != input.verified_dpop_jkt
    {
        return Err(OAuthError::InvalidGrant.into());
    }
    pkce::verify(&request.code_challenge, input.verifier)?;
    if code.consumed_at.is_some() {
        // Commit revocation on valid replay, rather than rolling it back with the error.
        sqlx::query("UPDATE identity_oauth_grants SET revoked_at=clock_timestamp() WHERE code_hash=$1 AND revoked_at IS NULL")
            .bind(hash(input.code)).execute(&mut *tx).await?;
        audit(&mut tx, code.principal_id, "identity.oauth.code_replayed").await?;
        tx.commit().await?;
        return Err(OAuthError::InvalidGrant.into());
    }
    if code.expires_at <= Utc::now() {
        return Err(OAuthError::InvalidGrant.into());
    }
    let active: Option<Uuid> = sqlx::query_scalar("SELECT s.id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.id=$1 AND s.principal_id=$2 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p")
        .bind(code.session_id).bind(code.principal_id).fetch_optional(&mut *tx).await?;
    if active.is_none() {
        return Err(OAuthError::InvalidGrant.into());
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_oauth_grants(id,code_hash,session_id,principal_id,client_id,parameters) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id).bind(hash(input.code)).bind(code.session_id).bind(code.principal_id)
        .bind(input.client_id).bind(code.parameters).execute(&mut *tx).await?;
    sqlx::query("UPDATE identity_oauth_codes SET consumed_at=clock_timestamp() WHERE code_hash=$1")
        .bind(hash(input.code))
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, code.principal_id, "identity.oauth.code_exchanged").await?;
    tx.commit().await?;
    Ok(AuthorizedGrant {
        id,
        principal_id: code.principal_id,
        session_id: code.session_id,
        request,
    })
}
