use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    oauth::{clients::ClientRegistry, codes::AuthorizedGrant},
    tokens_claims::{AccessTokenClaims, IdTokenClaims, JsonWebKeySet, ProofConfirmation, TokenSet},
    tokens_config::TokenConfig,
    tokens_error::TokenError,
    tokens_grants::{self, ActiveGrant},
    tokens_keys::KeyRing,
    tokens_policy,
};

#[path = "identity.tokens.refresh.rs"]
mod refresh;
pub use refresh::RefreshRequest;
#[path = "identity.tokens.exchange.rs"]
mod exchange;
pub use exchange::AuthorizationCodeRequest;
#[path = "identity.tokens.userinfo.rs"]
mod userinfo;

pub const ACCESS_TOKEN_TTL_SECONDS: u64 = 15 * 60;

pub struct TokenService {
    issuer: String,
    audiences: std::collections::BTreeSet<String>,
    keys: KeyRing,
}

impl TokenService {
    pub fn new(config: TokenConfig) -> Result<Self, TokenError> {
        let keys = KeyRing::new(&config)?;
        Ok(Self {
            issuer: config.issuer,
            audiences: config.allowed_audiences,
            keys,
        })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub(crate) fn endpoint(&self, path: &str) -> String {
        format!("{}/{path}", self.issuer.trim_end_matches('/'))
    }

    pub fn jwks(&self) -> JsonWebKeySet {
        self.keys.jwks(Utc::now().timestamp() as u64)
    }

    /// A code exchange capability may issue exactly one response. Authentication,
    /// audience, client and proof binding come exclusively from stored evidence.
    pub async fn issue_grant(
        &self,
        db: &PgPool,
        clients: &ClientRegistry,
        grant: &AuthorizedGrant,
    ) -> Result<TokenSet, TokenError> {
        let mut tx = db.begin().await?;
        let response = self.issue_grant_in(&mut tx, clients, grant).await?;
        tx.commit().await?;
        Ok(response)
    }

    async fn issue_grant_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        clients: &ClientRegistry,
        grant: &AuthorizedGrant,
    ) -> Result<TokenSet, TokenError> {
        let active = tokens_grants::load(tx, clients, grant.id).await?;
        if active.token_issued_at.is_some() {
            return Err(TokenError::InactiveGrant);
        }
        let request = &active.request;
        let response = self.sign_grant(&active)?;
        sqlx::query(
            "UPDATE identity_oauth_grants SET token_issued_at=clock_timestamp() WHERE id=$1",
        )
        .bind(active.id)
        .execute(&mut **tx)
        .await?;
        let refresh_token = if request_allows_refresh(request) {
            let value = crate::oauth::store::random_secret();
            let family_id = Uuid::new_v4();
            sqlx::query("INSERT INTO identity_oauth_refresh_tokens(token_hash,family_id,grant_id,principal_id,client_id) VALUES($1,$2,$3,$4,$5)")
                .bind(Sha256::digest(value.as_bytes()).to_vec())
                .bind(family_id)
                .bind(active.id)
                .bind(active.principal_id)
                .bind(request.client_id())
                .execute(&mut **tx)
                .await?;
            Some(value)
        } else {
            None
        };
        crate::oauth::store::audit(tx, active.principal_id, "identity.oauth.tokens_issued").await?;
        Ok(TokenSet {
            refresh_token,
            ..response
        })
    }

    pub(crate) fn sign_grant(&self, grant: &ActiveGrant) -> Result<TokenSet, TokenError> {
        let now = Utc::now();
        let issued_at = now.timestamp() as u64;
        let mut expires_at =
            (issued_at + ACCESS_TOKEN_TTL_SECONDS).min(grant.session_expires_at.timestamp() as u64);
        if let Some(deadline) = grant
            .request
            .minimum_authentication
            .deadline(&grant.authentication, now)?
        {
            expires_at = expires_at.min(deadline.timestamp() as u64);
        }
        if expires_at <= issued_at {
            return Err(TokenError::InactiveGrant);
        }
        let request = &grant.request;
        if !self.audiences.contains(request.audience()) {
            return Err(TokenError::InvalidPolicy);
        }
        // OIDC scopes authorize identity claims, not operations on resource APIs.
        if request.audience() == tokens_policy::USERINFO_AUDIENCE
            && request.resource() != self.endpoint("oauth/userinfo")
        {
            return Err(TokenError::InvalidPolicy);
        }
        let api_scope = tokens_policy::access_scope(request.audience(), request.scope())?;
        grant.authentication.validate(now)?;
        let amr = grant.authentication.amr(now);
        tokens_policy::validate_amr(&amr)?;
        let step_up = grant.authentication.fresh_step_up(now);
        let access = AccessTokenClaims {
            sub: grant.principal_id.to_string(),
            token_type: "access".into(),
            client_id: request.client_id().into(),
            scope: api_scope,
            amr: amr.clone(),
            auth_time: grant.authentication.authenticated_at.timestamp() as u64,
            step_up_time: step_up.map(|value| value.0),
            step_up_expires_at: step_up.map(|value| value.1),
            iss: self.issuer.clone(),
            aud: request.audience().into(),
            exp: expires_at,
            iat: issued_at,
            nbf: issued_at,
            jti: Uuid::new_v4().to_string(),
            sid: grant.session_id.to_string(),
            grant_id: grant.id.to_string(),
            cnf: request
                .dpop_jkt
                .clone()
                .map(|jkt| ProofConfirmation { jkt }),
        };
        let access_token = self.keys.sign("at+jwt", &access)?;
        let digest = Sha256::digest(access_token.as_bytes());
        let identity = IdTokenClaims {
            iss: self.issuer.clone(),
            sub: access.sub,
            aud: request.client_id().into(),
            exp: expires_at,
            iat: issued_at,
            auth_time: access.auth_time,
            amr,
            nonce: request.nonce.clone(),
            sid: access.sid,
            at_hash: URL_SAFE_NO_PAD.encode(&digest[..16]),
        };
        Ok(TokenSet {
            access_token,
            id_token: self.keys.sign("JWT", &identity)?,
            token_type: if access.cnf.is_some() {
                "DPoP"
            } else {
                "Bearer"
            },
            expires_in: expires_at - issued_at,
            scope: request.scope().into(),
            refresh_token: None,
        })
    }

    /// Cryptographic validation alone does not check session/grant revocation.
    pub fn verify(&self, token: &str, audience: &str) -> Result<AccessTokenClaims, TokenError> {
        if token.len() > 16_384 || !self.audiences.contains(audience) {
            return Err(TokenError::InvalidToken);
        }
        let now = Utc::now().timestamp() as u64;
        let header = decode_header(token)?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("at+jwt")
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(TokenError::InvalidToken);
        }
        let key = self
            .keys
            .decoding_key(header.kid.as_deref().ok_or(TokenError::InvalidToken)?, now)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "nbf"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let claims = decode::<AccessTokenClaims>(token, key, &validation)?.claims;
        if claims.token_type != "access"
            || claims.iat > now
            || claims.nbf != claims.iat
            || claims.exp <= now
            || claims.exp <= claims.iat
            || claims.exp - claims.iat > ACCESS_TOKEN_TTL_SECONDS
            || claims.auth_time > claims.iat
            || claims.client_id.is_empty()
            || Uuid::parse_str(&claims.sub).is_err()
            || Uuid::parse_str(&claims.sid).is_err()
            || Uuid::parse_str(&claims.grant_id).is_err()
            || Uuid::parse_str(&claims.jti).is_err()
            || claims
                .cnf
                .as_ref()
                .is_some_and(|c| !crate::oauth::pkce::is_sha256_base64url(&c.jkt))
        {
            return Err(TokenError::InvalidToken);
        }
        match (claims.step_up_time, claims.step_up_expires_at) {
            (None, None) => {}
            (Some(at), Some(until))
                if at >= claims.auth_time && at <= claims.iat && until > claims.iat => {}
            _ => return Err(TokenError::InvalidToken),
        }
        tokens_policy::validate_scopes(audience, &claims.scope)?;
        tokens_policy::validate_amr(&claims.amr)?;
        Ok(claims)
    }

    pub async fn introspect(
        &self,
        db: &PgPool,
        clients: &ClientRegistry,
        token: &str,
        audience: &str,
    ) -> Result<Option<AccessTokenClaims>, TokenError> {
        let Ok(claims) = self.verify(token, audience) else {
            return Ok(None);
        };
        let id = Uuid::parse_str(&claims.grant_id).map_err(|_| TokenError::InvalidToken)?;
        let mut tx = db.begin().await?;
        let grant = match tokens_grants::load(&mut tx, clients, id).await {
            Ok(grant) => grant,
            Err(TokenError::Database(error)) => return Err(error.into()),
            Err(_) => return Ok(None),
        };
        let active = grant.token_issued_at.is_some()
            && grant.session_id.to_string() == claims.sid
            && grant.principal_id.to_string() == claims.sub
            && grant.request.client_id() == claims.client_id
            && grant.request.audience() == claims.aud
            && grant.request.dpop_jkt.as_deref() == claims.cnf.as_ref().map(|c| c.jkt.as_str());
        tx.commit().await?;
        Ok(active.then_some(claims))
    }
}

fn request_allows_refresh(request: &crate::oauth::request::AuthorizationRequest) -> bool {
    request
        .scope()
        .split(' ')
        .any(|scope| scope == "offline_access")
}

#[cfg(test)]
#[path = "identity.tokens.tests.rs"]
pub(crate) mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.tokens.database.tests.rs"]
mod database_tests;
