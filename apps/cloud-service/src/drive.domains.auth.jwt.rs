use std::sync::OnceLock;
use std::time::{Duration, Instant};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::http::error::AppError;

const DRIVE_AUDIENCE: &str = "nvbes-cloud-service";
const JWKS_CACHE_TTL: Duration = Duration::from_secs(300);

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static JWKS_CACHE: OnceLock<RwLock<CachedJwks>> = OnceLock::new();

#[derive(Debug, Deserialize, Clone)]
pub struct IdentityJwtActorClaim {
    pub sub: String,
    pub client_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdentityJwtClaims {
    pub sub: String,
    pub workspace_id: Option<String>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub token_type: String,
    pub amr: Vec<String>,
    pub client_id: Option<String>,
    pub iss: String,
    pub aud: String,
    pub act: Option<IdentityJwtActorClaim>,
}

#[derive(Default)]
struct CachedJwks {
    fetched_at: Option<Instant>,
    keys: Vec<(String, DecodingKey)>,
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

#[derive(Debug, Deserialize)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
}

fn http_client() -> reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new).clone()
}

fn cache() -> &'static RwLock<CachedJwks> {
    JWKS_CACHE.get_or_init(|| RwLock::new(CachedJwks::default()))
}

fn identity_base_url() -> String {
    std::env::var("NVBES_IDENTITY_SERVICE_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
}

pub async fn verify_identity_access_token(token: &str) -> Result<IdentityJwtClaims, AppError> {
    let header = decode_header(token).map_err(|error| {
        AppError::unauthorized("invalid_token", format!("Invalid token header: {error}"))
    })?;
    if !matches!(header.alg, Algorithm::PS256 | Algorithm::RS256) {
        return Err(AppError::unauthorized(
            "invalid_token_algorithm",
            "Invalid token signing algorithm.",
        ));
    }

    let kid = header.kid.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_token",
            "Identity token is missing a key identifier.",
        )
    })?;
    let decoding_key = get_decoding_key(&kid).await?;

    let issuer = identity_base_url().trim_end_matches('/').to_string();
    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[&issuer]);
    validation.set_audience(&[DRIVE_AUDIENCE]);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    let data = decode::<IdentityJwtClaims>(token, &decoding_key, &validation).map_err(|error| {
        AppError::unauthorized(
            "invalid_token",
            format!("Identity token validation failed: {error}"),
        )
    })?;
    let claims = data.claims;
    validate_identity_claims(&claims, &issuer)?;
    Ok(claims)
}

fn validate_identity_claims(
    claims: &IdentityJwtClaims,
    expected_issuer: &str,
) -> Result<(), AppError> {
    if claims.token_type != "access"
        || claims.iss != expected_issuer
        || claims.aud != DRIVE_AUDIENCE
    {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The Identity access token is not valid for Drive.",
        ));
    }
    Ok(())
}

async fn get_decoding_key(kid: &str) -> Result<DecodingKey, AppError> {
    if let Some(key) = read_cached_key(kid).await {
        return Ok(key);
    }

    refresh_jwks().await?;
    read_cached_key(kid).await.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_token",
            "Unable to resolve the signing key for the Identity token.",
        )
    })
}

async fn read_cached_key(kid: &str) -> Option<DecodingKey> {
    let guard = cache().read().await;
    let cache_valid = guard
        .fetched_at
        .is_some_and(|fetched_at| fetched_at.elapsed() <= JWKS_CACHE_TTL);
    if !cache_valid {
        return None;
    }

    guard
        .keys
        .iter()
        .find(|(cached_kid, _)| cached_kid == kid)
        .map(|(_, key)| key.clone())
}

async fn refresh_jwks() -> Result<(), AppError> {
    let url = format!(
        "{}/.well-known/jwks.json",
        identity_base_url().trim_end_matches('/')
    );
    let request = http_client().get(url);
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|error| {
            AppError::internal(
                "jwks_fetch_failed",
                format!("Failed to fetch Identity JWKS: {error}"),
            )
        })?;
    if !response.status().is_success() {
        return Err(AppError::internal(
            "jwks_fetch_failed",
            format!(
                "Identity JWKS fetch failed with status {}",
                response.status()
            ),
        ));
    }

    let jwks = response.json::<JwksResponse>().await.map_err(|error| {
        AppError::internal(
            "jwks_invalid",
            format!("Identity JWKS payload is invalid: {error}"),
        )
    })?;

    let keys = jwks
        .keys
        .into_iter()
        .map(|key| {
            DecodingKey::from_rsa_components(&key.n, &key.e)
                .map(|decoding_key| (key.kid, decoding_key))
                .map_err(|error| {
                    AppError::internal(
                        "jwks_invalid",
                        format!("Identity JWKS contains an invalid RSA key: {error}"),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut guard = cache().write().await;
    guard.fetched_at = Some(Instant::now());
    guard.keys = keys;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DRIVE_AUDIENCE, IdentityJwtClaims, validate_identity_claims};

    const ISSUER: &str = "https://identity.nvbes.test";

    #[test]
    fn accepts_access_token_for_cloud_audience() {
        let claims = claims(DRIVE_AUDIENCE);

        validate_identity_claims(&claims, ISSUER).expect("Cloud access token should be accepted");
    }

    #[test]
    fn rejects_account_audience() {
        let claims = claims("nvbes-account-service");

        let error = validate_identity_claims(&claims, ISSUER)
            .expect_err("Account access token must not be accepted by Cloud");

        assert_eq!(error.code, "invalid_token");
    }

    #[test]
    fn rejects_non_access_token() {
        let mut claims = claims(DRIVE_AUDIENCE);
        claims.token_type = "id".to_string();

        let error = validate_identity_claims(&claims, ISSUER)
            .expect_err("ID token must not be accepted as an access token");

        assert_eq!(error.code, "invalid_token");
    }

    fn claims(audience: &str) -> IdentityJwtClaims {
        IdentityJwtClaims {
            sub: "principal-id".to_string(),
            workspace_id: Some("workspace-id".to_string()),
            tenant_id: Some("tenant-id".to_string()),
            organization_id: None,
            token_type: "access".to_string(),
            amr: vec!["pwd".to_string()],
            client_id: Some("cloud-web".to_string()),
            iss: ISSUER.to_string(),
            aud: audience.to_string(),
            act: None,
        }
    }
}
