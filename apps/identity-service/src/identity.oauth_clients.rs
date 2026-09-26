use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::hash_token;

const AUTHORIZATION_CODE_TTL_SECONDS: i64 = 600; // 10 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub is_confidential: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCodeInfo {
    pub code: String,
    pub client_id: String,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub redirect_uri: String,
    pub scope: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

pub struct CreateAuthorizationCodeParams<'a> {
    pub client_id: &'a str,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub redirect_uri: String,
    pub scope: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Get OAuth client by client_id
pub async fn get_oauth_client(db: &PgPool, client_id: &str) -> anyhow::Result<Option<OAuthClient>> {
    let row = sqlx::query!(
        "SELECT id, client_id, name, redirect_uris, scopes, is_confidential
         FROM identity_oauth_clients
         WHERE client_id = $1",
        client_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| OAuthClient {
        id: r.id,
        client_id: r.client_id,
        name: r.name,
        redirect_uris: r.redirect_uris,
        scopes: r.scopes,
        is_confidential: r.is_confidential,
    }))
}

/// Validate client credentials
pub async fn validate_client_credentials(
    db: &PgPool,
    client_id: &str,
    client_secret: &str,
) -> anyhow::Result<bool> {
    let client_secret_hash = hash_token(client_secret);

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM identity_oauth_clients
            WHERE client_id = $1 AND client_secret_hash = $2
        ) AS \"exists!\"",
        client_id,
        &client_secret_hash
    )
    .fetch_one(db)
    .await?;

    Ok(exists)
}

/// Check if redirect_uri is allowed for a client
pub async fn is_redirect_uri_allowed(
    db: &PgPool,
    client_id: &str,
    redirect_uri: &str,
) -> anyhow::Result<bool> {
    let allowed_uris = sqlx::query_scalar!(
        "SELECT redirect_uris FROM identity_oauth_clients WHERE client_id = $1",
        client_id
    )
    .fetch_one(db)
    .await?;

    Ok(allowed_uris.contains(&redirect_uri.to_string()))
}

/// Create an authorization code
pub async fn create_authorization_code(
    db: &PgPool,
    params: CreateAuthorizationCodeParams<'_>,
) -> anyhow::Result<String> {
    let code = random_token();
    let code_hash = hash_token(&code);
    let expires_at = Utc::now() + Duration::seconds(AUTHORIZATION_CODE_TTL_SECONDS);

    sqlx::query!(
        "INSERT INTO identity_authorization_codes (id, code_hash, client_id, principal_id, session_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        Uuid::new_v4(),
        &code_hash,
        params.client_id,
        params.principal_id,
        params.session_id,
        &params.redirect_uri,
        &params.scope,
        params.code_challenge.as_deref(),
        params.code_challenge_method.as_deref(),
        expires_at
    )
    .execute(db)
    .await?;

    Ok(code)
}

/// Validate and consume an authorization code
pub async fn validate_and_consume_authorization_code(
    db: &PgPool,
    code: &str,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: Option<&str>,
) -> anyhow::Result<AuthorizationCodeInfo> {
    let code_hash = hash_token(code);

    let row = sqlx::query!(
        "SELECT id, principal_id, session_id, redirect_uri, scope, code_challenge, code_challenge_method
         FROM identity_authorization_codes
         WHERE code_hash = $1 AND client_id = $2 AND consumed_at IS NULL AND expires_at > clock_timestamp()
         FOR UPDATE",
        &code_hash,
        client_id
    )
    .fetch_one(db)
    .await?;

    // Validate redirect_uri matches
    if row.redirect_uri != redirect_uri {
        anyhow::bail!("Redirect URI mismatch");
    }

    // Validate PKCE if present
    if let (Some(challenge), Some(method), Some(verifier)) = (
        row.code_challenge.as_ref(),
        row.code_challenge_method.as_ref(),
        code_verifier,
    ) {
        anyhow::ensure!(
            validate_pkce(challenge, method, verifier),
            "PKCE validation failed"
        );
    }

    // Consume the code
    sqlx::query!(
        "UPDATE identity_authorization_codes SET consumed_at = clock_timestamp() WHERE id = $1",
        row.id
    )
    .execute(db)
    .await?;

    Ok(AuthorizationCodeInfo {
        code: code.to_string(),
        client_id: client_id.to_string(),
        principal_id: row.principal_id,
        session_id: row.session_id,
        redirect_uri: row.redirect_uri,
        scope: row.scope,
        code_challenge: row.code_challenge,
        code_challenge_method: row.code_challenge_method,
    })
}

/// Validate PKCE code challenge
fn validate_pkce(challenge: &str, method: &str, verifier: &str) -> bool {
    match method {
        "plain" => challenge == verifier,
        "S256" => {
            use sha2::{Digest, Sha256};
            let hash = Sha256::digest(verifier.as_bytes());
            let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);
            challenge == encoded
        }
        _ => false,
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
#[path = "identity.oauth_clients.tests.rs"]
mod tests;
