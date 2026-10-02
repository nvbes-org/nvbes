//! Resource proofs use a configured public origin, never Host or proxy headers.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use http::{HeaderMap, Method, Uri};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use url::Url;

#[path = "dpop.resource.replay.rs"]
mod replay;

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("invalid resource authentication")]
    Invalid,
    #[error("resource proof verification unavailable")]
    Unavailable,
    #[error("invalid resource public origin")]
    Configuration,
}

pub struct Credentials<'a> {
    pub token: &'a str,
    pub dpop: bool,
    proof: Option<&'a str>,
}

impl<'a> Credentials<'a> {
    pub fn from_headers(headers: &'a HeaderMap) -> Result<Self, ResourceError> {
        let header = single(headers, "authorization")?.ok_or(ResourceError::Invalid)?;
        let (scheme, token) = header.split_once(' ').ok_or(ResourceError::Invalid)?;
        let token = token.trim_start_matches(' ');
        let dpop = scheme.eq_ignore_ascii_case("DPoP");
        if (!dpop && !scheme.eq_ignore_ascii_case("Bearer"))
            || token.is_empty()
            || token.len() > 16_384
            || token.bytes().any(|b| b.is_ascii_whitespace())
        {
            return Err(ResourceError::Invalid);
        }
        let proof = single(headers, "dpop")?;
        if dpop != proof.is_some() {
            return Err(ResourceError::Invalid);
        }
        Ok(Self { token, dpop, proof })
    }
}

fn single<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, ResourceError> {
    let mut values = headers.get_all(name).iter();
    let value = values
        .next()
        .map(|v| v.to_str())
        .transpose()
        .map_err(|_| ResourceError::Invalid)?;
    if values.next().is_some() {
        return Err(ResourceError::Invalid);
    }
    Ok(value)
}

pub fn binding(cnf: Option<&serde_json::Value>) -> Result<Option<&str>, ResourceError> {
    let Some(cnf) = cnf else { return Ok(None) };
    let object = cnf.as_object().ok_or(ResourceError::Invalid)?;
    let jkt = object
        .get("jkt")
        .and_then(|v| v.as_str())
        .ok_or(ResourceError::Invalid)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(jkt)
        .map_err(|_| ResourceError::Invalid)?;
    if object.len() != 1 || bytes.len() != 32 || URL_SAFE_NO_PAD.encode(bytes) != jkt {
        return Err(ResourceError::Invalid);
    }
    Ok(Some(jkt))
}

#[derive(Clone)]
pub struct ResourceVerifier {
    origin: String,
    permits: Arc<Semaphore>,
}

pub struct VerifiedResourceProof {
    jkt_hash: Vec<u8>,
    jti_hash: Vec<u8>,
    bucket: i16,
    expires_at: DateTime<Utc>,
    permits: Arc<Semaphore>,
}

impl ResourceVerifier {
    pub fn new(origin: &str) -> Result<Self, ResourceError> {
        let url = Url::parse(origin).map_err(|_| ResourceError::Configuration)?;
        let local = url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https" || local)
            || url.host_str().is_none()
            || origin.trim_end_matches('/') != url.origin().ascii_serialization()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ResourceError::Configuration);
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            permits: Arc::new(Semaphore::new(16)),
        })
    }

    pub fn verify(
        verifier: Option<&Self>,
        credentials: &Credentials<'_>,
        jkt: Option<&str>,
        method: &Method,
        uri: &Uri,
    ) -> Result<Option<VerifiedResourceProof>, ResourceError> {
        match (jkt, credentials.dpop, credentials.proof) {
            (None, false, None) => Ok(None),
            (Some(jkt), true, Some(proof)) => {
                let verifier = verifier.ok_or(ResourceError::Unavailable)?;
                if uri.scheme().is_some() || uri.authority().is_some() {
                    return Err(ResourceError::Invalid);
                }
                let expected_url = format!("{}{}", verifier.origin, uri.path());
                let verified = crate::verify_dpop_proof(
                    proof,
                    method.as_str(),
                    &expected_url,
                    Some(credentials.token),
                    300,
                )
                .map_err(|_| ResourceError::Invalid)?;
                if crate::jwk_thumbprint(&verified.jwk) != jkt
                    || verified.claims.jti.is_empty()
                    || verified.claims.jti.len() > 256
                {
                    return Err(ResourceError::Invalid);
                }
                let jkt_hash = Sha256::digest(jkt.as_bytes()).to_vec();
                let jti_hash = Sha256::digest(verified.claims.jti.as_bytes()).to_vec();
                let bucket = i16::from(
                    Sha256::digest([jkt_hash.as_slice(), jti_hash.as_slice()].concat())[0] % 64,
                );
                Ok(Some(VerifiedResourceProof {
                    jkt_hash,
                    jti_hash,
                    bucket,
                    expires_at: DateTime::from_timestamp(verified.claims.iat + 301, 0)
                        .ok_or(ResourceError::Invalid)?,
                    permits: verifier.permits.clone(),
                }))
            }
            _ => Err(ResourceError::Invalid),
        }
    }
}

impl VerifiedResourceProof {
    /// Consume before entering the business handler; retries need a new proof.
    pub async fn consume(self, db: &sqlx::PgPool, token_exp: u64) -> Result<(), ResourceError> {
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| ResourceError::Unavailable)?;
        tokio::time::timeout(
            Duration::from_secs(2),
            replay::consume(db, &self, token_exp),
        )
        .await
        .map_err(|_| ResourceError::Unavailable)?
    }
}

#[cfg(test)]
#[path = "dpop.resource.tests.rs"]
mod tests;
