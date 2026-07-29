use jsonwebtoken::{self as jwt, Algorithm, TokenData, Validation};
use nvbes_redis::connection::RedisPool;

use super::types::{JwtService, TokenClaims};
use crate::http::error::AppError;

const CLOUD_TOKEN_AUDIENCE: &str = "nvbes-cloud-service";
const JWT_CLOCK_SKEW_SECONDS: i64 = 5;

impl JwtService {
    pub fn decode_token(&self, token: &str, expected_type: &str) -> Result<TokenClaims, AppError> {
        let header = jwt::decode_header(token).map_err(|e| {
            AppError::unauthorized("invalid_token", format!("Invalid token header: {}", e))
        })?;
        if !matches!(header.alg, Algorithm::PS256 | Algorithm::RS256) {
            return Err(AppError::unauthorized(
                "invalid_token_algorithm",
                "Invalid token signing algorithm",
            ));
        }
        require_token_type_header(header.typ.as_deref(), expected_type)?;
        let decoding_key = header
            .kid
            .as_ref()
            .and_then(|kid| {
                self.decoding_keys
                    .iter()
                    .find(|(k, _)| k == kid)
                    .map(|(_, dk)| dk.clone())
            })
            .ok_or_else(|| {
                AppError::unauthorized(
                    "invalid_token_key",
                    "The token kid is missing or is not an active verification key.",
                )
            })?;
        let mut validation = Validation::new(header.alg);
        let audiences = [self.audience.as_str(), CLOUD_TOKEN_AUDIENCE];
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&audiences);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = JWT_CLOCK_SKEW_SECONDS as u64;
        validation.set_required_spec_claims(&[
            "jti",
            "sid",
            "sub",
            "token_type",
            "scope",
            "iss",
            "aud",
            "exp",
            "iat",
            "nbf",
        ]);
        let token_data: TokenData<TokenClaims> = jwt::decode(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::unauthorized("token_expired", "Token has expired")
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    AppError::unauthorized("invalid_token", "Invalid token format")
                }
                _ => AppError::unauthorized("token_validation_failed", format!("{}", e)),
            })?;
        let claims = token_data.claims;
        if claims.token_type != expected_type {
            return Err(AppError::unauthorized(
                "invalid_token_type",
                format!("Expected {}, got {}", expected_type, claims.token_type),
            ));
        }
        if claims.iss != self.issuer {
            return Err(AppError::unauthorized(
                "invalid_issuer",
                "Invalid token issuer",
            ));
        }
        if !audiences.iter().any(|audience| claims.aud == *audience) {
            return Err(AppError::unauthorized(
                "invalid_audience",
                "Invalid token audience",
            ));
        }
        validate_claim_lifetime(
            &claims,
            match expected_type {
                "access" => self.access_token_expiry.num_seconds(),
                "refresh" => self.refresh_token_expiry.num_seconds(),
                _ => 0,
            },
        )?;
        Ok(claims)
    }

    pub fn decode_token_ignore_expiry(&self, token: &str) -> Result<TokenClaims, AppError> {
        let header = jwt::decode_header(token).map_err(|e| {
            AppError::unauthorized("invalid_token", format!("Invalid token header: {}", e))
        })?;
        if !matches!(header.alg, Algorithm::PS256 | Algorithm::RS256) {
            return Err(AppError::unauthorized(
                "invalid_token_algorithm",
                "Invalid token signing algorithm",
            ));
        }
        require_token_type_header(header.typ.as_deref(), "access")?;
        let decoding_key = header
            .kid
            .as_ref()
            .and_then(|kid| {
                self.decoding_keys
                    .iter()
                    .find(|(k, _)| k == kid)
                    .map(|(_, dk)| dk.clone())
            })
            .ok_or_else(|| {
                AppError::unauthorized(
                    "invalid_token_key",
                    "The token kid is missing or is not an active verification key.",
                )
            })?;
        let mut validation = Validation::new(header.alg);
        let audiences = [self.audience.as_str(), CLOUD_TOKEN_AUDIENCE];
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&audiences);
        validation.validate_exp = false;
        validation.validate_nbf = true;
        validation.leeway = JWT_CLOCK_SKEW_SECONDS as u64;
        validation.set_required_spec_claims(&[
            "jti",
            "sid",
            "sub",
            "token_type",
            "scope",
            "iss",
            "aud",
            "exp",
            "iat",
            "nbf",
        ]);
        let token_data: TokenData<TokenClaims> = jwt::decode(token, &decoding_key, &validation)
            .map_err(|e| AppError::unauthorized("invalid_token", format!("{}", e)))?;
        let claims = token_data.claims;
        if claims.token_type != "access" {
            return Err(AppError::unauthorized(
                "invalid_token_type",
                format!("Expected access, got {}", claims.token_type),
            ));
        }
        if claims.iss != self.issuer {
            return Err(AppError::unauthorized(
                "invalid_issuer",
                "Invalid token issuer",
            ));
        }
        if !audiences.iter().any(|audience| claims.aud == *audience) {
            return Err(AppError::unauthorized(
                "invalid_audience",
                "Invalid token audience",
            ));
        }
        validate_claim_lifetime(&claims, self.access_token_expiry.num_seconds())?;
        Ok(claims)
    }

    pub async fn validate_token(
        &self,
        redis: Option<&RedisPool>,
        token: &str,
        expected_type: &str,
    ) -> Result<TokenClaims, AppError> {
        let claims = self.decode_token(token, expected_type)?;
        if expected_type == "refresh"
            && let Some(redis) = redis
        {
            validate_refresh_token_registration(redis, &claims.jti).await?;
        }
        Ok(claims)
    }
}

fn require_token_type_header(
    actual: Option<&str>,
    expected_claim_type: &str,
) -> Result<(), AppError> {
    let expected = match expected_claim_type {
        "access" => "at+jwt",
        "refresh" => "refresh+jwt",
        _ => {
            return Err(AppError::unauthorized(
                "invalid_token_type",
                "The expected token type is unsupported.",
            ));
        }
    };
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid_token_type",
            "The JWT typ header is missing or does not match the token use.",
        ))
    }
}

fn validate_claim_lifetime(claims: &TokenClaims, maximum_ttl: i64) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();
    if maximum_ttl <= 0
        || claims.jti.trim().is_empty()
        || claims.sid.trim().is_empty()
        || claims.sub.trim().is_empty()
        || claims.iat > now + JWT_CLOCK_SKEW_SECONDS
        || claims.nbf > claims.iat + JWT_CLOCK_SKEW_SECONDS
        || claims.exp <= claims.iat
        || claims.exp > claims.iat + maximum_ttl + JWT_CLOCK_SKEW_SECONDS
    {
        return Err(AppError::unauthorized(
            "invalid_token_claims",
            "The JWT required claims or lifetime are invalid.",
        ));
    }
    Ok(())
}

async fn validate_refresh_token_registration(redis: &RedisPool, jti: &str) -> Result<(), AppError> {
    let token = nvbes_redis::refresh_token::get_refresh_token(redis, jti)
        .await
        .map_err(|e| AppError::internal("refresh_token_lookup_failed", format!("{}", e)))?;
    let token = token.ok_or_else(|| {
        AppError::unauthorized(
            "refresh_token_not_registered",
            "Refresh token is not registered",
        )
    })?;
    if token.revoked_at.is_some() || token.reuse_detected_at.is_some() {
        return Err(AppError::unauthorized(
            "refresh_token_revoked",
            "Refresh token has been revoked",
        ));
    }
    Ok(())
}
