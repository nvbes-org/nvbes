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
    let row = sqlx::query_as::<_, (Uuid, String, String, Vec<String>, Vec<String>, bool)>(
        "SELECT id, client_id, name, redirect_uris, scopes, is_confidential
         FROM identity_oauth_clients
         WHERE client_id = $1",
    )
    .bind(client_id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(
        |(id, client_id, name, redirect_uris, scopes, is_confidential)| OAuthClient {
            id,
            client_id,
            name,
            redirect_uris,
            scopes,
            is_confidential,
        },
    ))
}

/// Validate client credentials
pub async fn validate_client_credentials(
    db: &PgPool,
    client_id: &str,
    client_secret: &str,
) -> anyhow::Result<bool> {
    let client_secret_hash = hash_token(client_secret);

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM identity_oauth_clients
            WHERE client_id = $1 AND client_secret_hash = $2
        )",
    )
    .bind(client_id)
    .bind(&client_secret_hash)
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
    let allowed_uris: Vec<String> =
        sqlx::query_scalar("SELECT redirect_uris FROM identity_oauth_clients WHERE client_id = $1")
            .bind(client_id)
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

    sqlx::query(
        "INSERT INTO identity_authorization_codes (id, code_hash, client_id, principal_id, session_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(Uuid::new_v4())
    .bind(&code_hash)
    .bind(params.client_id)
    .bind(params.principal_id)
    .bind(params.session_id)
    .bind(&params.redirect_uri)
    .bind(&params.scope)
    .bind(&params.code_challenge)
    .bind(&params.code_challenge_method)
    .bind(expires_at)
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

    let (
        auth_code_id,
        principal_id,
        session_id,
        stored_redirect_uri,
        scope,
        code_challenge,
        code_challenge_method,
    ): (
        Uuid,
        Uuid,
        Uuid,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT id, principal_id, session_id, redirect_uri, scope, code_challenge, code_challenge_method
         FROM identity_authorization_codes
         WHERE code_hash = $1 AND client_id = $2 AND consumed_at IS NULL AND expires_at > clock_timestamp()
         FOR UPDATE",
    )
    .bind(&code_hash)
    .bind(client_id)
    .fetch_one(db)
    .await?;

    // Validate redirect_uri matches
    if stored_redirect_uri != redirect_uri {
        anyhow::bail!("Redirect URI mismatch");
    }

    // Validate PKCE if present
    if let (Some(challenge), Some(method), Some(verifier)) = (
        code_challenge.as_ref(),
        code_challenge_method.as_ref(),
        code_verifier,
    ) {
        anyhow::ensure!(
            validate_pkce(challenge, method, verifier),
            "PKCE validation failed"
        );
    }

    // Consume the code
    sqlx::query(
        "UPDATE identity_authorization_codes SET consumed_at = clock_timestamp() WHERE id = $1",
    )
    .bind(auth_code_id)
    .execute(db)
    .await?;

    Ok(AuthorizationCodeInfo {
        code: code.to_string(),
        client_id: client_id.to_string(),
        principal_id,
        session_id,
        redirect_uri: stored_redirect_uri,
        scope,
        code_challenge,
        code_challenge_method,
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
