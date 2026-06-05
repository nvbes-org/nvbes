use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

use super::{http::fetch_federation_url, types::FederatedIdentityProviderRecord};
use crate::http::error::AppError;

#[derive(Debug, Deserialize)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    #[serde(default)]
    pub jwks_uri: Option<String>,
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    #[serde(default)]
    pub subject_types_supported: Vec<String>,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Vec<String>,
    #[serde(default)]
    pub claims_supported: Vec<String>,
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct OidcJwks {
    pub keys: Vec<OidcJwk>,
}

#[derive(Debug, Deserialize)]
pub struct OidcJwk {
    pub kid: Option<String>,
    pub kty: String,
    pub alg: Option<String>,
    #[serde(default, rename = "use")]
    pub key_use: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OidcAudience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct OidcIdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: OidcAudience,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub azp: Option<String>,
}

impl OidcIdTokenClaims {
    pub fn username(&self) -> String {
        self.preferred_username
            .clone()
            .unwrap_or_else(|| self.sub.clone())
    }

    pub fn audience_contains(&self, client_id: &str) -> bool {
        match &self.aud {
            OidcAudience::Single(value) => value == client_id,
            OidcAudience::Multiple(values) => values.iter().any(|value| value == client_id),
        }
    }

    pub fn is_multi_audience(&self) -> bool {
        matches!(self.aud, OidcAudience::Multiple(_))
    }
}

pub async fn fetch_oidc_discovery(
    provider: &FederatedIdentityProviderRecord,
    strict_mode: bool,
) -> Result<OidcDiscoveryDocument, AppError> {
    let discovery_url = oidc_discovery_url(provider);
    let response = fetch_federation_url(&discovery_url, strict_mode).await?;
    response
        .json::<OidcDiscoveryDocument>()
        .await
        .map_err(|err| {
            AppError::bad_request(
                crate::domains::federation::contract::OIDC_DISCOVERY_INVALID,
                &format!("{}", err),
            )
        })
}

pub fn oidc_discovery_url(provider: &FederatedIdentityProviderRecord) -> String {
    if let Some(metadata_url) = &provider.metadata_url {
        metadata_url.clone()
    } else if let Some(issuer) = &provider.issuer {
        format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        )
    } else {
        String::new()
    }
}

pub async fn validate_oidc_id_token(
    provider: &FederatedIdentityProviderRecord,
    id_token: &str,
    expected_nonce: Option<&str>,
    strict_mode: bool,
) -> Result<OidcIdTokenClaims, AppError> {
    let discovery = fetch_oidc_discovery(provider, strict_mode).await?;
    let client_id = provider.client_id.as_ref().ok_or_else(|| {
        AppError::bad_request(
            "client_id_missing",
            "The OIDC provider is missing a configured client_id.",
        )
    })?;
    let header = decode_header(id_token).map_err(|err| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_ID_TOKEN,
            &format!("{}", err),
        )
    })?;
    let algorithm = match header.alg {
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => header.alg,
        _ => {
            return Err(AppError::bad_request(
                crate::domains::federation::contract::INVALID_ID_TOKEN,
                "The ID token signing algorithm is not allowed.",
            ));
        }
    };
    let kid = header.kid.ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_ID_TOKEN,
            "The ID token is missing a key id.",
        )
    })?;
    let jwks_uri = discovery.jwks_uri.ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::JWKS_MISSING,
            "The OIDC discovery document does not expose a JWKS URI.",
        )
    })?;
    let jwks = fetch_federation_url(&jwks_uri, strict_mode)
        .await?
        .json::<OidcJwks>()
        .await
        .map_err(|err| AppError::bad_request("jwks_invalid", &format!("{}", err)))?;
    let key = jwks
        .keys
        .into_iter()
        .find(|key| key.kid.as_deref() == Some(&kid))
        .ok_or_else(|| {
            AppError::bad_request("jwk_not_found", "No matching signing key was found.")
        })?;
    if key.kty != "RSA" {
        return Err(AppError::bad_request(
            "invalid_jwk",
            "The JWKS key type is not supported.",
        ));
    }
    if key.key_use.as_deref().is_some_and(|value| value != "sig") {
        return Err(AppError::bad_request(
            "invalid_jwk",
            "The JWKS key is not intended for signatures.",
        ));
    }
    if key
        .alg
        .as_deref()
        .is_some_and(|value| value != algorithm_name(algorithm))
    {
        return Err(AppError::bad_request(
            "invalid_jwk",
            "The JWKS key algorithm does not match the token header.",
        ));
    }
    let decoding_key = DecodingKey::from_rsa_components(
        &key.n.ok_or_else(|| {
            AppError::bad_request("invalid_jwk", "The JWKS key is missing modulus data.")
        })?,
        &key.e.ok_or_else(|| {
            AppError::bad_request("invalid_jwk", "The JWKS key is missing exponent data.")
        })?,
    )
    .map_err(|err| AppError::bad_request("invalid_jwk", &format!("{}", err)))?;
    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[discovery.issuer.as_str()]);
    validation.set_audience(&[client_id.as_str()]);
    let claims = decode::<OidcIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|err| {
            AppError::bad_request(
                crate::domains::federation::contract::INVALID_ID_TOKEN,
                &format!("{}", err),
            )
        })?
        .claims;
    let _ = (&claims.iss, &claims.azp);
    let _ = claims.audience_contains(client_id);
    let _ = claims.is_multi_audience();
    if let Some(expected_nonce) = expected_nonce {
        if claims.nonce.as_deref() != Some(expected_nonce) {
            return Err(AppError::bad_request(
                crate::domains::federation::contract::INVALID_ID_TOKEN,
                "The ID token nonce does not match the request.",
            ));
        }
    }
    Ok(claims)
}

fn algorithm_name(algorithm: Algorithm) -> &'static str {
    match algorithm {
        Algorithm::RS256 => "RS256",
        Algorithm::RS384 => "RS384",
        Algorithm::RS512 => "RS512",
        _ => "",
    }
}

#[cfg(test)]
#[path = "identity.domains.federation.oidc.tests.rs"]
mod tests;
