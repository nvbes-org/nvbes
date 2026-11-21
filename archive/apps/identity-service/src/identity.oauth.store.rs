use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{clients::ClientRegistry, error::OAuthError, request::AuthorizationRequest};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Protocol(#[from] OAuthError),
    #[error("protocol persistence failed")]
    Database(#[from] sqlx::Error),
    #[error("protocol state serialization failed")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Clone, Copy)]
pub enum RequestKind {
    Authorization,
    Par,
}

impl RequestKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Authorization => "authorization",
            Self::Par => "par",
        }
    }
}

pub async fn create_request(
    db: &PgPool,
    request: &AuthorizationRequest,
    kind: RequestKind,
) -> Result<String, StoreError> {
    let mut tx = db.begin().await?;
    let handle = create_request_in(&mut tx, request, kind).await?;
    tx.commit().await?;
    Ok(handle)
}

pub(super) async fn create_request_in(
    tx: &mut Transaction<'_, Postgres>,
    request: &AuthorizationRequest,
    kind: RequestKind,
) -> Result<String, StoreError> {
    let handle = random_secret();
    sqlx::query("INSERT INTO identity_oauth_requests(handle_hash,kind,client_id,parameters) VALUES($1,$2,$3,$4)")
        .bind(hash(&handle)).bind(kind.as_str()).bind(&request.client_id)
        .bind(serde_json::to_value(request)?).execute(&mut **tx).await?;
    Ok(handle)
}

/// Consume PAR and replace it with a hosted authorization transaction atomically.
/// A mismatched client never consumes someone else's request URI.
pub async fn consume_par(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    client_id: &str,
) -> Result<String, StoreError> {
    let mut tx = db.begin().await?;
    let next = consume_par_in(&mut tx, clients, handle, client_id).await?;
    tx.commit().await?;
    Ok(next)
}

pub(super) async fn consume_par_in(
    tx: &mut Transaction<'_, Postgres>,
    clients: &ClientRegistry,
    handle: &str,
    client_id: &str,
) -> Result<String, StoreError> {
    let value: serde_json::Value = sqlx::query_scalar("DELETE FROM identity_oauth_requests WHERE handle_hash=$1 AND kind='par' AND client_id=$2 AND expires_at>clock_timestamp() RETURNING parameters")
        .bind(hash(handle)).bind(client_id).fetch_optional(&mut **tx).await?
        .ok_or(OAuthError::InvalidRequest)?;
    let request = AuthorizationRequest::restore(value, clients)?;
    let next = random_secret();
    sqlx::query("INSERT INTO identity_oauth_requests(handle_hash,kind,client_id,parameters) VALUES($1,'authorization',$2,$3)")
        .bind(hash(&next)).bind(&request.client_id).bind(serde_json::to_value(&request)?)
        .execute(&mut **tx).await?;
    Ok(next)
}

pub async fn load_request(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
) -> Result<AuthorizationRequest, StoreError> {
    let value: serde_json::Value = sqlx::query_scalar("SELECT parameters FROM identity_oauth_requests WHERE handle_hash=$1 AND kind='authorization' AND expires_at>clock_timestamp()")
        .bind(hash(handle)).fetch_optional(db).await?.ok_or(OAuthError::InvalidRequest)?;
    Ok(AuthorizationRequest::restore(value, clients)?)
}

pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    event: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO identity_audit_events(id,principal_id,actor_principal_id,event_type,correlation_id) VALUES($1,$2,$2,$3,$4)")
        .bind(Uuid::new_v4()).bind(principal_id).bind(event).bind(Uuid::new_v4())
        .execute(&mut **tx).await?;
    Ok(())
}

pub(crate) fn random_secret() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn hash(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}
