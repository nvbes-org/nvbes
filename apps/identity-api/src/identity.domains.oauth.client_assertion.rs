use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode_header};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;

use super::service::{ClientAssertionAuthentication, ClientAuthentication};

pub const CLIENT_ASSERTION_TYPE_JWT_BEARER: &str =
    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";
const MAX_ASSERTION_TTL_SECONDS: i64 = 300;
const CLOCK_SKEW_SECONDS: i64 = 5;

#[derive(Debug, Deserialize)]
struct ClientAssertionIdentityClaims {
    iss: String,
    sub: String,
}

#[derive(Debug, Deserialize)]
struct ClientAssertionClaims {
    iss: String,
    sub: String,
    exp: i64,
    #[serde(default)]
    iat: Option<i64>,
    #[serde(default)]
    jti: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClientAssertionJwk {
    kty: String,
    #[serde(default)]
    kid: Option<String>,
    #[serde(default)]
    alg: Option<String>,
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

pub fn client_id_from_unverified_assertion(
    body_client_id: Option<&str>,
    assertion: &str,
) -> Result<String, AppError> {
    let token =
        jsonwebtoken::dangerous::insecure_decode::<ClientAssertionIdentityClaims>(assertion)
            .map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion is invalid.")
            })?;
    let client_id = token.claims.iss.trim();
    if client_id.is_empty() || token.claims.sub != token.claims.iss {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion issuer and subject must identify the same client.",
        ));
    }

    if body_client_id.is_some_and(|value| value != client_id) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication credentials are inconsistent.",
        ));
    }

    Ok(client_id.to_string())
}

pub async fn verify_private_key_jwt(
    db: &sqlx::PgPool,
    auth: &ClientAuthentication,
    expected_audience: &str,
) -> Result<bool, AppError> {
    let Some(assertion) = auth.client_assertion.as_ref() else {
        return Ok(false);
    };

    if assertion.assertion_type != CLIENT_ASSERTION_TYPE_JWT_BEARER {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Unsupported client_assertion_type.",
        ));
    }

    let client = sqlx::query(
        r#"
        SELECT
            tenant_id,
            client_assertion_public_key_jwk,
            revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&auth.client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::unauthorized("invalid_client", "The OAuth client is not registered.")
    })?;

    if client
        .get::<Option<DateTime<Utc>>, _>("revoked_at")
        .is_some()
    {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The OAuth client has been revoked.",
        ));
    }

    let tenant_id: Uuid = client.get("tenant_id");
    let jwk_value: Option<serde_json::Value> = client.get("client_assertion_public_key_jwk");
    let jwk_value = jwk_value.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "This OAuth client is not configured for private_key_jwt.",
        )
    })?;
    let jwk: ClientAssertionJwk = serde_json::from_value(jwk_value).map_err(|_| {
        AppError::unauthorized(
            "invalid_client",
            "The OAuth client assertion key is invalid.",
        )
    })?;

    let header = decode_header(&assertion.assertion).map_err(|_| {
        AppError::unauthorized("invalid_client", "The client_assertion is invalid.")
    })?;
    let algorithm = supported_algorithm(header.alg)?;
    validate_jwk_header_consistency(&jwk, header.kid.as_deref(), algorithm)?;
    let decoding_key = decoding_key_for_jwk(&jwk, algorithm)?;

    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[auth.client_id.as_str()]);
    validation.sub = Some(auth.client_id.clone());
    validation.set_audience(&[expected_audience]);
    validation.validate_nbf = true;
    validation.leeway = CLOCK_SKEW_SECONDS as u64;
    validation.set_required_spec_claims(&["iss", "sub", "aud", "exp"]);

    let claims = jsonwebtoken::decode::<ClientAssertionClaims>(
        &assertion.assertion,
        &decoding_key,
        &validation,
    )
    .map_err(|_| AppError::unauthorized("invalid_client", "The client_assertion is invalid."))?
    .claims;

    validate_claims(&claims, &auth.client_id)?;
    record_assertion_jti(
        db,
        tenant_id,
        &auth.client_id,
        claims.jti.as_deref(),
        claims.exp,
    )
    .await?;

    Ok(true)
}

fn supported_algorithm(algorithm: Algorithm) -> Result<Algorithm, AppError> {
    match algorithm {
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 | Algorithm::ES256 => Ok(algorithm),
        _ => Err(AppError::unauthorized(
            "invalid_client",
            "Unsupported client_assertion signing algorithm.",
        )),
    }
}

fn validate_jwk_header_consistency(
    jwk: &ClientAssertionJwk,
    header_kid: Option<&str>,
    algorithm: Algorithm,
) -> Result<(), AppError> {
    if let (Some(header_kid), Some(jwk_kid)) = (header_kid, jwk.kid.as_deref()) {
        if header_kid != jwk_kid {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The client_assertion kid does not match the registered client key.",
            ));
        }
    }

    if let Some(jwk_alg) = jwk.alg.as_deref() {
        if algorithm_from_name(jwk_alg) != Some(algorithm) {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The client_assertion alg does not match the registered client key.",
            ));
        }
    }

    Ok(())
}

fn decoding_key_for_jwk(
    jwk: &ClientAssertionJwk,
    algorithm: Algorithm,
) -> Result<DecodingKey, AppError> {
    match (jwk.kty.as_str(), algorithm) {
        ("RSA", Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512) => {
            let n = jwk.n.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })?;
            let e = jwk.e.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })?;
            DecodingKey::from_rsa_components(n, e).map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })
        }
        ("EC", Algorithm::ES256) => {
            if jwk.crv.as_deref() != Some("P-256") {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "Only P-256 client_assertion EC keys are supported.",
                ));
            }
            let x = jwk.x.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })?;
            let y = jwk.y.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })?;
            DecodingKey::from_ec_components(x, y).map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })
        }
        _ => Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion key type is not compatible with the signing algorithm.",
        )),
    }
}

fn validate_claims(claims: &ClientAssertionClaims, client_id: &str) -> Result<(), AppError> {
    if claims.iss != client_id || claims.sub != client_id {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion issuer and subject must match the authenticated client.",
        ));
    }

    let now = Utc::now().timestamp();
    if claims.exp > now + MAX_ASSERTION_TTL_SECONDS {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion expiration is too far in the future.",
        ));
    }

    if claims.iat.is_some_and(|iat| iat > now + CLOCK_SKEW_SECONDS) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion was issued in the future.",
        ));
    }

    let jti = claims.jti.as_deref().map(str::trim).unwrap_or("");
    if jti.is_empty() {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion jti is required.",
        ));
    }

    Ok(())
}

async fn record_assertion_jti(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
    jti: Option<&str>,
    exp: i64,
) -> Result<(), AppError> {
    let jti = jti.unwrap_or_default().trim();
    let expires_at = DateTime::<Utc>::from_timestamp(exp, 0).ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "The client_assertion expiration is invalid.",
        )
    })?;

    sqlx::query("DELETE FROM oauth_client_assertion_jtis WHERE expires_at < NOW()")
        .execute(db)
        .await?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO oauth_client_assertion_jtis (tenant_id, client_id, jti, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (client_id, jti) DO NOTHING
        RETURNING jti
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(jti)
    .bind(expires_at)
    .fetch_optional(db)
    .await?;

    if inserted.is_none() {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion has already been used.",
        ));
    }

    Ok(())
}

fn algorithm_from_name(value: &str) -> Option<Algorithm> {
    match value {
        "RS256" => Some(Algorithm::RS256),
        "RS384" => Some(Algorithm::RS384),
        "RS512" => Some(Algorithm::RS512),
        "ES256" => Some(Algorithm::ES256),
        _ => None,
    }
}

pub fn assertion_auth(assertion_type: &str, assertion: &str) -> ClientAssertionAuthentication {
    ClientAssertionAuthentication {
        assertion_type: assertion_type.to_string(),
        assertion: assertion.to_string(),
    }
}

pub fn is_supported_client_assertion_public_jwk(value: &serde_json::Value) -> bool {
    let Ok(jwk) = serde_json::from_value::<ClientAssertionJwk>(value.clone()) else {
        return false;
    };
    let Some(algorithm) = jwk.alg.as_deref().and_then(algorithm_from_name) else {
        return false;
    };

    supported_algorithm(algorithm).is_ok() && decoding_key_for_jwk(&jwk, algorithm).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        algorithm_from_name, is_supported_client_assertion_public_jwk, supported_algorithm,
    };
    use jsonwebtoken::Algorithm;

    #[test]
    fn algorithm_policy_accepts_private_key_jwt_algorithms_only() {
        assert!(supported_algorithm(Algorithm::RS256).is_ok());
        assert!(supported_algorithm(Algorithm::ES256).is_ok());
        assert!(supported_algorithm(Algorithm::HS256).is_err());
    }

    #[test]
    fn algorithm_names_are_strict() {
        assert_eq!(algorithm_from_name("RS256"), Some(Algorithm::RS256));
        assert_eq!(algorithm_from_name("none"), None);
    }

    #[test]
    fn registered_public_jwk_requires_supported_key_shape() {
        assert!(!is_supported_client_assertion_public_jwk(
            &serde_json::json!({"kty": "oct", "alg": "HS256", "k": "secret"})
        ));
    }
}
