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
    validation.leeway = super::CLOCK_SKEW_SECONDS as u64;
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
