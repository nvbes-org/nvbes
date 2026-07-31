use chrono::{DateTime, Utc};
use jsonwebtoken::{Validation, decode_header};
use sqlx::Row;
use uuid::Uuid;

use crate::domains::oauth::service::ClientAuthentication;
use crate::http::error::AppError;

use super::{
    CLIENT_ASSERTION_TYPE_JWT_BEARER, ClientAssertionClaims, ClientAssertionJwk,
    jwk::{decoding_key_for_jwk, supported_algorithm, validate_jwk_header_consistency},
    replay::{record_assertion_jti, validate_claims},
};

pub async fn verify_private_key_jwt(
    db: &sqlx::PgPool,
    auth: &ClientAuthentication,
    endpoint_audience: &str,
    issuer_audience: &str,
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
            security_profile::text AS security_profile,
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

    let header = decode_header(&assertion.assertion).map_err(|_| {
        AppError::unauthorized("invalid_client", "The client_assertion is invalid.")
    })?;
    let tenant_id: Uuid = client.get("tenant_id");
    let high_assurance = client.get::<String, _>("security_profile") == "high_assurance";
    let expected_audience = if high_assurance {
        issuer_audience
    } else {
        endpoint_audience
    };
    let jwks = crate::domains::oauth::clients::keys::active_jwks(
        db,
        tenant_id,
        &auth.client_id,
        crate::domains::oauth::clients::keys::OAuthClientKeyPurpose::ClientAuthentication,
    )
    .await?;
    let jwk_value = select_assertion_jwk(&jwks, header.kid.as_deref(), high_assurance)?;
    let jwk: ClientAssertionJwk = serde_json::from_value(jwk_value.clone()).map_err(|_| {
        AppError::unauthorized(
            "invalid_client",
            "The OAuth client assertion key is invalid.",
        )
    })?;
    let algorithm = supported_algorithm(header.alg)?;
    validate_jwk_header_consistency(&jwk, header.kid.as_deref(), algorithm)?;
    let decoding_key = decoding_key_for_jwk(&jwk, algorithm)?;

    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[auth.client_id.as_str()]);
    validation.sub = Some(auth.client_id.clone());
    validation.set_audience(&[expected_audience]);
    validation.validate_nbf = true;
    validation.leeway = super::CLOCK_SKEW_SECONDS as u64;
    validation.set_required_spec_claims(&["iss", "sub", "aud", "exp"]);

    let claims = jsonwebtoken::decode::<ClientAssertionClaims>(
        &assertion.assertion,
        &decoding_key,
        &validation,
    )
    .map_err(|_| AppError::unauthorized("invalid_client", "The client_assertion is invalid."))?
    .claims;

    if high_assurance && claims.aud.as_str() != Some(issuer_audience) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "High-assurance client_assertion aud must be the issuer as a JSON string.",
        ));
    }
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

fn select_assertion_jwk<'a>(
    jwks: &'a serde_json::Value,
    header_kid: Option<&str>,
    high_assurance: bool,
) -> Result<&'a serde_json::Value, AppError> {
    let keys = jwks
        .get("keys")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| invalid_client_key("The registered client JWKS is invalid."))?;
    if high_assurance && header_kid.is_none_or(str::is_empty) {
        return Err(invalid_client_key(
            "High-assurance client assertions require a kid.",
        ));
    }
    match header_kid {
        Some(kid) => keys
            .iter()
            .find(|jwk| jwk.get("kid").and_then(serde_json::Value::as_str) == Some(kid))
            .ok_or_else(|| invalid_client_key("The client_assertion kid is not active.")),
        None if keys.len() == 1 => Ok(&keys[0]),
        None => Err(invalid_client_key(
            "The client_assertion kid is required when multiple keys are active.",
        )),
    }
}

fn invalid_client_key(message: &'static str) -> AppError {
    AppError::unauthorized("invalid_client", message)
}
