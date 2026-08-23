use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode_header};
use serde::Deserialize;
use std::collections::HashSet;

use super::RequestObjectClaims;
use crate::http::error::AppError;

const MAX_REQUEST_OBJECT_TTL_SECONDS: i64 = 3_600;
const MAX_REQUEST_OBJECT_AGE_SECONDS: i64 = 3_600;
const REQUEST_OBJECT_CLOCK_SKEW_SECONDS: i64 = 60;
const REQUEST_OBJECT_TYP: &str = "oauth-authz-req+jwt";
const MAX_REQUEST_OBJECT_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestObjectJwks {
    keys: Vec<RequestObjectJwk>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestObjectJwk {
    kty: String,
    kid: String,
    alg: String,
    #[serde(default)]
    r#use: Option<String>,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
    #[serde(default)]
    crv: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
}

pub fn validate_high_assurance_jwks(jwks: &serde_json::Value) -> Result<(), AppError> {
    let jwks: RequestObjectJwks = serde_json::from_value(jwks.clone()).map_err(|_| {
        invalid_request_object("request_object_signing_jwks must be a valid JWKS object.")
    })?;
    if jwks.keys.is_empty() || jwks.keys.len() > 10 {
        return Err(invalid_request_object(
            "request_object_signing_jwks must contain between one and ten keys.",
        ));
    }
    let mut kids = HashSet::with_capacity(jwks.keys.len());
    for jwk in &jwks.keys {
        if !kids.insert(jwk.kid.as_str()) {
            return Err(invalid_request_object(
                "Every request-object JWK must use a unique kid.",
            ));
        }
        validate_high_assurance_jwk(jwk)?;
    }
    Ok(())
}

pub fn validate_high_assurance_request_object(
    request_jwt: &str,
    jwks: &serde_json::Value,
    client_id: &str,
    issuer_url: &str,
) -> Result<RequestObjectClaims, AppError> {
    if request_jwt.len() > MAX_REQUEST_OBJECT_BYTES {
        return Err(invalid_request_object(
            "The request object exceeds the 16 KiB limit.",
        ));
    }
    let header = decode_header(request_jwt)
        .map_err(|_| invalid_request_object("The request object header is invalid."))?;
    if header
        .typ
        .as_deref()
        .is_some_and(|value| value != REQUEST_OBJECT_TYP)
    {
        return Err(invalid_request_object(
            "When present, the request object typ must be oauth-authz-req+jwt.",
        ));
    }
    if header.jku.is_some()
        || header.jwk.is_some()
        || header.x5u.is_some()
        || header.crit.as_ref().is_some_and(|crit| !crit.is_empty())
    {
        return Err(invalid_request_object(
            "Request objects cannot select remote, embedded, or unsupported critical keys.",
        ));
    }
    let kid = header
        .kid
        .as_deref()
        .filter(|kid| !kid.is_empty())
        .ok_or_else(|| invalid_request_object("The request object kid is required."))?;
    let algorithm = match header.alg {
        Algorithm::PS256 | Algorithm::ES256 | Algorithm::EdDSA => header.alg,
        _ => {
            return Err(invalid_request_object(
                "High-assurance request objects must use PS256, ES256, or EdDSA.",
            ));
        }
    };

    let jwks: RequestObjectJwks = serde_json::from_value(jwks.clone())
        .map_err(|_| invalid_request_object("The registered request-object JWKS is invalid."))?;
    let jwk = jwks
        .keys
        .iter()
        .find(|jwk| jwk.kid == kid)
        .ok_or_else(|| invalid_request_object("The request object kid is not registered."))?;
    validate_high_assurance_jwk(jwk)?;
    if algorithm_name(algorithm) != jwk.alg {
        return Err(invalid_request_object(
            "The request object alg does not match the registered JWK.",
        ));
    }

    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[client_id]);
    validation.set_audience(&[issuer_url]);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.leeway = REQUEST_OBJECT_CLOCK_SKEW_SECONDS as u64;
    validation.set_required_spec_claims(&["iss", "aud", "exp", "nbf"]);

    let claims = jsonwebtoken::decode::<RequestObjectClaims>(
        request_jwt,
        &decoding_key_for_request_object(jwk, algorithm)?,
        &validation,
    )
    .map_err(|_| invalid_request_object("The request object signature or claims are invalid."))?
    .claims;
    validate_request_object_claims(&claims, client_id, issuer_url)?;
    Ok(claims)
}

pub async fn record_request_object_jti(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    client_id: &str,
    claims: &RequestObjectClaims,
) -> Result<(), AppError> {
    let Some(jti) = claims
        .jti
        .as_deref()
        .map(str::trim)
        .filter(|jti| !jti.is_empty())
    else {
        return Ok(());
    };
    let expires_at = chrono::DateTime::<Utc>::from_timestamp(claims.exp, 0)
        .ok_or_else(|| invalid_request_object("The request object exp is invalid."))?;
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tx,
        nvbes_tenancy::RlsContext {
            tenant_id: Some(tenant_id),
            ..Default::default()
        },
    )
    .await?;
    sqlx::query(
        "DELETE FROM oauth_request_object_jtis WHERE tenant_id = $1 AND expires_at < NOW()",
    )
    .bind(tenant_id)
    .execute(&mut *tx)
    .await?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO oauth_request_object_jtis (tenant_id, client_id, jti, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (client_id, jti) DO NOTHING
        RETURNING jti
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(jti)
    .bind(expires_at)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(invalid_request_object(
            "The request object has already been used.",
        ));
    }
    tx.commit().await?;
    Ok(())
}

fn validate_high_assurance_jwk(jwk: &RequestObjectJwk) -> Result<(), AppError> {
    if jwk.kid.trim().is_empty() || jwk.r#use.as_deref().is_some_and(|value| value != "sig") {
        return Err(invalid_request_object(
            "Every request-object JWK requires a kid and use=sig when use is present.",
        ));
    }
    match (jwk.kty.as_str(), jwk.alg.as_str()) {
        ("RSA", "PS256") => {
            let modulus = jwk.n.as_deref().ok_or_else(|| {
                invalid_request_object("The PS256 request-object JWK is missing n.")
            })?;
            let exponent = jwk.e.as_deref().ok_or_else(|| {
                invalid_request_object("The PS256 request-object JWK is missing e.")
            })?;
            let modulus = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(modulus)
                .map_err(|_| invalid_request_object("The RSA JWK modulus is invalid."))?;
            let exponent = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(exponent)
                .map_err(|_| invalid_request_object("The RSA JWK exponent is invalid."))?;
            if modulus.len() < 256 || exponent.is_empty() {
                return Err(invalid_request_object(
                    "PS256 request-object keys must contain an RSA modulus of at least 2048 bits.",
                ));
            }
        }
        ("EC", "ES256") if jwk.crv.as_deref() == Some("P-256") => {
            let x = decode_p256_coordinate(jwk.x.as_deref(), "x")?;
            let y = decode_p256_coordinate(jwk.y.as_deref(), "y")?;
            if x.len() != 32 || y.len() != 32 {
                return Err(invalid_request_object(
                    "ES256 request-object coordinates must be exactly 32 bytes.",
                ));
            }
        }
        ("OKP", "EdDSA") if jwk.crv.as_deref() == Some("Ed25519") => {
            let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(jwk.x.as_deref().unwrap_or_default())
                .map_err(|_| invalid_request_object("The Ed25519 JWK x value is invalid."))?;
            if x.len() != 32 {
                return Err(invalid_request_object(
                    "Ed25519 request-object keys must contain a 32-byte x value.",
                ));
            }
        }
        _ => {
            return Err(invalid_request_object(
                "Request-object JWKs must use PS256, ES256, or EdDSA with Ed25519.",
            ));
        }
    }
    Ok(())
}

fn decode_p256_coordinate(
    coordinate: Option<&str>,
    name: &'static str,
) -> Result<Vec<u8>, AppError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(coordinate.unwrap_or_default())
        .map_err(|_| {
            invalid_request_object(match name {
                "x" => "The EC JWK x coordinate is invalid.",
                _ => "The EC JWK y coordinate is invalid.",
            })
        })
}

fn decoding_key_for_request_object(
    jwk: &RequestObjectJwk,
    algorithm: Algorithm,
) -> Result<DecodingKey, AppError> {
    match algorithm {
        Algorithm::PS256 => DecodingKey::from_rsa_components(
            jwk.n.as_deref().unwrap_or_default(),
            jwk.e.as_deref().unwrap_or_default(),
        )
        .map_err(|_| invalid_request_object("The registered RSA JWK is invalid.")),
        Algorithm::ES256 => DecodingKey::from_ec_components(
            jwk.x.as_deref().unwrap_or_default(),
            jwk.y.as_deref().unwrap_or_default(),
        )
        .map_err(|_| invalid_request_object("The registered EC JWK is invalid.")),
        Algorithm::EdDSA => DecodingKey::from_ed_components(jwk.x.as_deref().unwrap_or_default())
            .map_err(|_| invalid_request_object("The registered Ed25519 JWK is invalid.")),
        _ => Err(invalid_request_object(
            "The request object algorithm is unsupported.",
        )),
    }
}

fn validate_request_object_claims(
    claims: &RequestObjectClaims,
    client_id: &str,
    issuer_url: &str,
) -> Result<(), AppError> {
    if claims.client_id.as_deref() != Some(client_id)
        || claims.response_type.as_deref() != Some("code")
        || !claims.aud.contains(issuer_url)
    {
        return Err(invalid_request_object(
            "The request object client_id, audience, or response_type is invalid.",
        ));
    }
    let now = Utc::now().timestamp();
    let nbf = claims
        .nbf
        .ok_or_else(|| invalid_request_object("The request object nbf is required."))?;
    if nbf > now + REQUEST_OBJECT_CLOCK_SKEW_SECONDS
        || nbf < now - MAX_REQUEST_OBJECT_AGE_SECONDS
        || claims.exp <= nbf
        || claims.exp > nbf + MAX_REQUEST_OBJECT_TTL_SECONDS
    {
        return Err(invalid_request_object(
            "The request object nbf and exp must define a lifetime of at most one hour.",
        ));
    }
    if claims.jti.as_deref().is_some_and(|jti| {
        let length = jti.trim().len();
        length == 0 || length > 255
    }) {
        return Err(invalid_request_object(
            "When present, request object jti must not be empty or exceed 255 bytes.",
        ));
    }
    Ok(())
}

fn algorithm_name(algorithm: Algorithm) -> &'static str {
    match algorithm {
        Algorithm::PS256 => "PS256",
        Algorithm::ES256 => "ES256",
        Algorithm::EdDSA => "EdDSA",
        _ => "",
    }
}

fn invalid_request_object(message: &'static str) -> AppError {
    AppError::bad_request("invalid_request_object", message)
}
