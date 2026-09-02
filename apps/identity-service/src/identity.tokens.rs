use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, traits::PublicKeyParts};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth;
use crate::tokens_config::TokenConfig;

const ACCESS_TOKEN_TTL_SECONDS: u64 = 15 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub token_type: String,
    pub scope: String,
    pub amr: Vec<String>,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub nbf: u64,
    pub jti: String,
    pub sid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonWebKeySet {
    pub keys: Vec<JsonWebKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonWebKey {
    pub kid: String,
    pub kty: &'static str,
    #[serde(rename = "use")]
    pub usage: &'static str,
    pub alg: &'static str,
    pub n: String,
    pub e: String,
}

pub struct TokenService {
    config: TokenConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    jwks: JsonWebKeySet,
}

#[derive(Debug, Serialize)]
pub struct SyntheticTokenResult {
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub algorithm: &'static str,
    pub key_id: String,
    pub expires_in_seconds: u64,
    pub active_before_revocation: bool,
    pub inactive_for_wrong_audience: bool,
    pub inactive_after_revocation: bool,
}

impl TokenService {
    pub fn new(config: TokenConfig) -> anyhow::Result<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes())?;
        let decoding_key = DecodingKey::from_rsa_pem(config.public_key_pem.as_bytes())?;
        let public_key = RsaPublicKey::from_public_key_pem(&config.public_key_pem)?;
        let jwks = JsonWebKeySet {
            keys: vec![JsonWebKey {
                kid: config.key_id.clone(),
                kty: "RSA",
                usage: "sig",
                alg: "RS256",
                n: URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
                e: URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
            }],
        };
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            jwks,
        })
    }

    pub fn jwks(&self) -> &JsonWebKeySet {
        &self.jwks
    }

    pub fn issue(
        &self,
        principal_id: Uuid,
        session_id: Uuid,
        audience: &str,
        scope: &str,
        amr: Vec<String>,
    ) -> anyhow::Result<String> {
        if !self.config.allowed_audiences.contains(audience) {
            anyhow::bail!("token audience is not allowed");
        }
        validate_scope(scope)?;
        let issued_at = Utc::now().timestamp() as u64;
        let claims = AccessTokenClaims {
            sub: principal_id.to_string(),
            token_type: "access".into(),
            scope: scope.into(),
            amr,
            iss: self.config.issuer.clone(),
            aud: audience.into(),
            exp: issued_at + ACCESS_TOKEN_TTL_SECONDS,
            iat: issued_at,
            nbf: issued_at,
            jti: Uuid::new_v4().to_string(),
            sid: session_id.to_string(),
        };
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.config.key_id.clone());
        header.typ = Some("at+jwt".into());
        Ok(encode(&header, &claims, &self.encoding_key)?)
    }

    pub fn verify(
        &self,
        token: &str,
        expected_audience: &str,
    ) -> anyhow::Result<AccessTokenClaims> {
        if !self.config.allowed_audiences.contains(expected_audience) {
            anyhow::bail!("token audience is not allowed");
        }
        let header = decode_header(token)?;
        if header.alg != Algorithm::RS256
            || header.kid.as_deref() != Some(&self.config.key_id)
            || header.typ.as_deref() != Some("at+jwt")
        {
            anyhow::bail!("token header is invalid");
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[expected_audience]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "nbf"]);
        let claims = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)?.claims;
        if claims.token_type != "access" {
            anyhow::bail!("token type is invalid");
        }
        Ok(claims)
    }

    pub async fn introspect(
        &self,
        db: &PgPool,
        token: &str,
        expected_audience: &str,
    ) -> anyhow::Result<Option<AccessTokenClaims>> {
        let Ok(claims) = self.verify(token, expected_audience) else {
            return Ok(None);
        };
        let Ok(session_id) = Uuid::parse_str(&claims.sid) else {
            return Ok(None);
        };
        let Ok(principal_id) = Uuid::parse_str(&claims.sub) else {
            return Ok(None);
        };
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM identity_sessions s JOIN identity_principals p ON p.id = s.principal_id WHERE s.id = $1 AND s.principal_id = $2 AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp() AND p.status = 'active')",
        )
        .bind(session_id)
        .bind(principal_id)
        .fetch_one(db)
        .await?;
        Ok(active.then_some(claims))
    }
}

pub async fn run_synthetic_smoke(
    db: &PgPool,
    service: &TokenService,
    email: &str,
    password: &str,
    audience: &str,
) -> anyhow::Result<SyntheticTokenResult> {
    let principal_id = auth::create_synthetic_identity(db, email, password).await?;
    let session_token = auth::authenticate(db, email, password).await?;
    let session_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM identity_sessions WHERE token_hash = $1 AND principal_id = $2",
    )
    .bind(auth::hash_token(&session_token))
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    let access_token = service.issue(
        principal_id,
        session_id,
        audience,
        "account:read",
        vec!["pwd".into()],
    )?;
    let active_before_revocation = service
        .introspect(db, &access_token, audience)
        .await?
        .is_some();
    let inactive_for_wrong_audience = service
        .introspect(db, &access_token, "unregistered-audience")
        .await?
        .is_none();
    sqlx::query("UPDATE identity_sessions SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(session_id)
        .execute(db)
        .await?;
    let inactive_after_revocation = service
        .introspect(db, &access_token, audience)
        .await?
        .is_none();
    let key = &service.jwks().keys[0];
    Ok(SyntheticTokenResult {
        principal_id,
        session_id,
        algorithm: key.alg,
        key_id: key.kid.clone(),
        expires_in_seconds: ACCESS_TOKEN_TTL_SECONDS,
        active_before_revocation,
        inactive_for_wrong_audience,
        inactive_after_revocation,
    })
}

fn validate_scope(scope: &str) -> anyhow::Result<()> {
    let valid = !scope.is_empty()
        && scope.len() <= 1024
        && scope.split(' ').all(|item| {
            !item.is_empty()
                && item.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_' | b'.')
                })
        });
    if !valid {
        anyhow::bail!("token scope is invalid")
    }
    Ok(())
}

#[cfg(test)]
#[path = "identity.tokens.tests.rs"]
mod tests;
