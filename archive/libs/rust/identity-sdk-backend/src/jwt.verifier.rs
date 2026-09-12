use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::SdkError;

const DEFAULT_JWKS_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityJwtActorClaim {
    pub sub: String,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityAccessTokenClaims {
    pub sub: String,
    pub workspace_id: Option<String>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub token_type: String,
    pub scope: String,
    #[serde(default)]
    pub amr: Vec<String>,
    pub client_id: Option<String>,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub act: Option<IdentityJwtActorClaim>,
}

impl IdentityAccessTokenClaims {
    pub fn require_scopes(&self, required: &[&str]) -> Result<(), SdkError> {
        let granted = self.scope.split_ascii_whitespace().collect::<Vec<_>>();
        if let Some(missing) = required
            .iter()
            .find(|required_scope| !granted.contains(required_scope))
        {
            return Err(SdkError::invalid_token(format!(
                "required OAuth scope `{missing}` is missing"
            )));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct IdentityJwtVerifier {
    issuer: String,
    audience: String,
    jwks_uri: String,
    http_client: reqwest::Client,
    cache_ttl: Duration,
    cache: Arc<RwLock<CachedJwks>>,
}

impl IdentityJwtVerifier {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Result<Self, SdkError> {
        let issuer = normalize_issuer(issuer.into())?;
        let audience = audience.into();
        if audience.trim().is_empty() {
            return Err(SdkError::Config(
                "Identity JWT audience cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            jwks_uri: format!("{issuer}/.well-known/jwks.json"),
            issuer,
            audience,
            http_client: reqwest::Client::new(),
            cache_ttl: DEFAULT_JWKS_CACHE_TTL,
            cache: Arc::new(RwLock::new(CachedJwks::default())),
        })
    }

    pub async fn verify_access_token(
        &self,
        token: &str,
    ) -> Result<IdentityAccessTokenClaims, SdkError> {
        let header = decode_header(token)
            .map_err(|error| SdkError::invalid_token(format!("invalid JWT header: {error}")))?;
        if !matches!(header.alg, Algorithm::PS256 | Algorithm::RS256) {
            return Err(SdkError::invalid_token(
                "token signing algorithm is not allowed",
            ));
        }
        let kid = header
            .kid
            .filter(|kid| !kid.trim().is_empty())
            .ok_or_else(|| SdkError::invalid_token("token key identifier is missing"))?;
        let decoding_key = self.decoding_key(&kid).await?;

        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let claims = decode::<IdentityAccessTokenClaims>(token, &decoding_key, &validation)
            .map_err(|error| SdkError::invalid_token(error.to_string()))?
            .claims;
        self.validate_claims(&claims)?;
        Ok(claims)
    }

    fn validate_claims(&self, claims: &IdentityAccessTokenClaims) -> Result<(), SdkError> {
        if claims.token_type != "access" {
            return Err(SdkError::invalid_token(
                "only Identity access tokens are accepted",
            ));
        }
        if claims.iss != self.issuer {
            return Err(SdkError::invalid_token(
                "token issuer does not match Identity",
            ));
        }
        if claims.aud != self.audience {
            return Err(SdkError::invalid_token(
                "token audience does not match this resource server",
            ));
        }
        if claims.sub.trim().is_empty() {
            return Err(SdkError::invalid_token("token subject is missing"));
        }
        Ok(())
    }

    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, SdkError> {
        if let Some(key) = self.cached_key(kid).await {
            return Ok(key);
        }

        self.refresh_jwks().await?;
        self.cached_key(kid).await.ok_or_else(|| {
            SdkError::invalid_token("token signing key is not present in Identity JWKS")
        })
    }

    async fn cached_key(&self, kid: &str) -> Option<DecodingKey> {
        let cache = self.cache.read().await;
        if cache
            .fetched_at
            .is_none_or(|fetched_at| fetched_at.elapsed() > self.cache_ttl)
        {
            return None;
        }
        cache
            .keys
            .iter()
            .find(|key| key.kid == kid)
            .map(|key| key.decoding_key.clone())
    }

    async fn refresh_jwks(&self) -> Result<(), SdkError> {
        let response = self
            .http_client
            .get(&self.jwks_uri)
            .send()
            .await
            .map_err(|error| SdkError::jwks(format!("request failed: {error}")))?;
        if !response.status().is_success() {
            return Err(SdkError::jwks(format!(
                "Identity returned HTTP {}",
                response.status()
            )));
        }
        let payload = response
            .json::<JwksResponse>()
            .await
            .map_err(|error| SdkError::jwks(format!("invalid response: {error}")))?;
        if payload.keys.is_empty() {
            return Err(SdkError::jwks("Identity returned an empty key set"));
        }

        let keys = payload
            .keys
            .into_iter()
            .map(CachedKey::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let mut cache = self.cache.write().await;
        cache.keys = keys;
        cache.fetched_at = Some(Instant::now());
        Ok(())
    }
}

#[derive(Default)]
struct CachedJwks {
    fetched_at: Option<Instant>,
    keys: Vec<CachedKey>,
}

struct CachedKey {
    kid: String,
    decoding_key: DecodingKey,
}

impl TryFrom<JwkKey> for CachedKey {
    type Error = SdkError;

    fn try_from(key: JwkKey) -> Result<Self, Self::Error> {
        if key.kty != "RSA" {
            return Err(SdkError::jwks(format!(
                "key `{}` uses unsupported type `{}`",
                key.kid, key.kty
            )));
        }
        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|error| SdkError::jwks(format!("key `{}` is invalid: {error}", key.kid)))?;
        Ok(Self {
            kid: key.kid,
            decoding_key,
        })
    }
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

#[derive(Deserialize)]
struct JwkKey {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

fn normalize_issuer(value: String) -> Result<String, SdkError> {
    let issuer = value.trim().trim_end_matches('/').to_string();
    let url = reqwest::Url::parse(&issuer)
        .map_err(|error| SdkError::Config(format!("Identity issuer is invalid: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(SdkError::Config(
            "Identity issuer must be an absolute HTTP(S) URL".to_string(),
        ));
    }
    Ok(issuer)
}

#[cfg(test)]
mod tests {
    use super::{IdentityAccessTokenClaims, IdentityJwtVerifier};

    fn claims() -> IdentityAccessTokenClaims {
        IdentityAccessTokenClaims {
            sub: "principal-id".to_string(),
            workspace_id: None,
            tenant_id: None,
            organization_id: None,
            token_type: "access".to_string(),
            scope: "account:profile:read account:privacy:read".to_string(),
            amr: vec!["pwd".to_string()],
            client_id: Some("account-web".to_string()),
            iss: "https://identity.example".to_string(),
            aud: "nvbes-account-service".to_string(),
            exp: 2,
            iat: 1,
            act: None,
        }
    }

    #[test]
    fn exact_resource_server_claims_are_accepted() {
        let verifier =
            IdentityJwtVerifier::new("https://identity.example/", "nvbes-account-service")
                .expect("valid verifier");

        verifier
            .validate_claims(&claims())
            .expect("matching access token claims");
    }

    #[test]
    fn another_audience_is_rejected() {
        let verifier =
            IdentityJwtVerifier::new("https://identity.example", "nvbes-account-service")
                .expect("valid verifier");
        let mut foreign = claims();
        foreign.aud = "nvbes-cloud-service".to_string();

        let error = verifier
            .validate_claims(&foreign)
            .expect_err("foreign audience must be rejected");

        assert!(error.to_string().contains("audience"));
    }

    #[test]
    fn id_tokens_are_rejected() {
        let verifier =
            IdentityJwtVerifier::new("https://identity.example", "nvbes-account-service")
                .expect("valid verifier");
        let mut id_token = claims();
        id_token.token_type = "id".to_string();

        let error = verifier
            .validate_claims(&id_token)
            .expect_err("ID token must be rejected");

        assert!(error.to_string().contains("access tokens"));
    }

    #[test]
    fn required_scopes_use_exact_tokens() {
        let claims = claims();
        claims
            .require_scopes(&["account:profile:read"])
            .expect("exact scope is granted");

        let error = claims
            .require_scopes(&["account:profile"])
            .expect_err("scope prefixes must not match");

        assert!(error.to_string().contains("account:profile"));
    }
}
