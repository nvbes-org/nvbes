use jsonwebtoken::{self as jwt, Algorithm, TokenData, Validation};
use nvbes_redis::connection::RedisPool;

use super::types::{JwtService, TokenClaims};
use crate::http::error::AppError;

const DRIVE_TOKEN_AUDIENCE: &str = "nvbes-drive-api";

impl JwtService {
    pub fn decode_token(&self, token: &str, expected_type: &str) -> Result<TokenClaims, AppError> {
        let header = jwt::decode_header(token).map_err(|e| {
            AppError::unauthorized("invalid_token", format!("Invalid token header: {}", e))
        })?;
        if header.alg != Algorithm::RS256 {
            return Err(AppError::unauthorized(
                "invalid_token_algorithm",
                "Invalid token signing algorithm",
            ));
        }
        let decoding_key = header
            .kid
            .as_ref()
            .and_then(|kid| {
                self.decoding_keys
                    .iter()
                    .find(|(k, _)| k == kid)
                    .map(|(_, dk)| dk.clone())
            })
            .or_else(|| self.decoding_keys.first().map(|(_, dk)| dk.clone()))
            .ok_or_else(|| AppError::internal("no_decoding_key", "No decoding key available"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        let audiences = [self.audience.as_str(), DRIVE_TOKEN_AUDIENCE];
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&audiences);
        validation.validate_exp = true;
        validation.validate_nbf = true;
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
        Ok(claims)
    }

    pub fn decode_token_ignore_expiry(&self, token: &str) -> Result<TokenClaims, AppError> {
        let header = jwt::decode_header(token).map_err(|e| {
            AppError::unauthorized("invalid_token", format!("Invalid token header: {}", e))
        })?;
        if header.alg != Algorithm::RS256 {
            return Err(AppError::unauthorized(
                "invalid_token_algorithm",
                "Invalid token signing algorithm",
            ));
        }
        let decoding_key = header
            .kid
            .as_ref()
            .and_then(|kid| {
                self.decoding_keys
                    .iter()
                    .find(|(k, _)| k == kid)
                    .map(|(_, dk)| dk.clone())
            })
            .or_else(|| self.decoding_keys.first().map(|(_, dk)| dk.clone()))
            .ok_or_else(|| AppError::internal("no_decoding_key", "No decoding key available"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        let audiences = [self.audience.as_str(), DRIVE_TOKEN_AUDIENCE];
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&audiences);
        validation.validate_exp = false;
        validation.validate_nbf = true;
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
