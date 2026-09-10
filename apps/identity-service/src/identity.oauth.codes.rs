use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    clients::ClientRegistry,
    error::OAuthError,
    pkce,
    request::AuthorizationRequest,
    session::ActiveSession,
    store::{StoreError, audit, hash, random_secret},
};

pub struct AuthorizationCode {
    pub code: String,
    pub request: AuthorizationRequest,
}

pub struct AuthorizedGrant {
    pub(crate) id: Uuid,
}

impl AuthorizedGrant {
    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// Only the consent module can reach this after locking the browser transaction
/// and active session. Request consumption, consent and code share one commit.
pub(super) async fn issue_code(
    tx: &mut Transaction<'_, Postgres>,
    request: AuthorizationRequest,
    session: &ActiveSession,
) -> Result<AuthorizationCode, StoreError> {
    let value = serde_json::to_value(&request)?;
    let authentication = serde_json::to_value(&session.authentication)?;
    let code = random_secret();
    sqlx::query("INSERT INTO identity_oauth_codes(code_hash,session_id,principal_id,client_id,parameters,authentication) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(hash(&code)).bind(session.id).bind(session.principal_id).bind(&request.client_id).bind(value).bind(authentication)
        .execute(&mut **tx).await?;
    audit(tx, session.principal_id, "identity.oauth.authorized").await?;
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
    let result = exchange_in(&mut tx, clients, input).await?;
    tx.commit().await?;
    match result {
        ExchangeOutcome::Granted(grant) => Ok(grant),
        ExchangeOutcome::Replayed => Err(OAuthError::InvalidGrant.into()),
    }
}

pub(crate) enum ExchangeOutcome {
    Granted(AuthorizedGrant),
    Replayed,
}

pub(crate) async fn exchange_in(
    tx: &mut Transaction<'_, Postgres>,
    clients: &ClientRegistry,
    input: CodeExchange<'_>,
) -> Result<ExchangeOutcome, StoreError> {
    let code: StoredCode = sqlx::query_as("SELECT principal_id,session_id,parameters,consumed_at,expires_at FROM identity_oauth_codes WHERE code_hash=$1 AND client_id=$2 FOR UPDATE")
        .bind(hash(input.code)).bind(input.client_id).fetch_optional(&mut **tx).await?
        .ok_or(OAuthError::InvalidGrant)?;
    let request = AuthorizationRequest::restore(code.parameters.clone(), clients)?;
    if request.redirect_uri != input.redirect_uri
        || request.dpop_jkt.as_deref() != input.verified_dpop_jkt
    {
        return Err(OAuthError::InvalidGrant.into());
    }
    pkce::verify(&request.code_challenge, input.verifier)?;
    if code.consumed_at.is_some() {
        // Return an outcome so callers can commit revocation and proof consumption
        // together before returning the protocol error.
        sqlx::query("UPDATE identity_oauth_grants SET revoked_at=clock_timestamp() WHERE code_hash=$1 AND revoked_at IS NULL")
            .bind(hash(input.code)).execute(&mut **tx).await?;
        audit(tx, code.principal_id, "identity.oauth.code_replayed").await?;
        return Ok(ExchangeOutcome::Replayed);
    }
    if code.expires_at <= Utc::now() {
        return Err(OAuthError::InvalidGrant.into());
    }
    let active: Option<Uuid> = sqlx::query_scalar("SELECT s.id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.id=$1 AND s.principal_id=$2 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p")
        .bind(code.session_id).bind(code.principal_id).fetch_optional(&mut **tx).await?;
    if active.is_none() {
        return Err(OAuthError::InvalidGrant.into());
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_oauth_grants(id,code_hash,session_id,principal_id,client_id,parameters) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id).bind(hash(input.code)).bind(code.session_id).bind(code.principal_id)
        .bind(input.client_id).bind(code.parameters).execute(&mut **tx).await?;
    sqlx::query("UPDATE identity_oauth_codes SET consumed_at=clock_timestamp() WHERE code_hash=$1")
        .bind(hash(input.code))
        .execute(&mut **tx)
        .await?;
    audit(tx, code.principal_id, "identity.oauth.code_exchanged").await?;
    Ok(ExchangeOutcome::Granted(AuthorizedGrant { id }))
}
