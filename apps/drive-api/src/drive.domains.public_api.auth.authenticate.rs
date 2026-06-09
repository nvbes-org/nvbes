use sqlx::PgPool;
use std::time::Duration;

use crate::http::{error::AppError, request::bearer_token};

use super::super::{db, errors::PublicApiErrorKind, http_signatures, types::PublicApiContext};
use super::scopes::key_hash;

#[path = "drive.domains.public_api.auth.authenticate.api_key.rs"]
mod api_key;
#[path = "drive.domains.public_api.auth.authenticate.m2m.rs"]
mod m2m;

pub(super) enum PublicApiCredential {
    Bearer(String),
    HttpSignature(String),
}

pub(super) async fn enforce_plan_rate_limit(
    redis: &nvbes_redis::RedisPool,
    action: &str,
    subject_key: &str,
    plan_code: &str,
) -> Result<(), AppError> {
    let limits = db::rate_limits_for_plan(plan_code);
    nvbes_core::limiter::check_rate_limit_pair(
        redis,
        action,
        nvbes_core::limiter::RateLimitRule {
            key: &format!("minute:{subject_key}"),
            max_hits: limits.requests_per_minute,
            window: Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("day:{subject_key}"),
            max_hits: limits.requests_per_day,
            window: Duration::from_secs(86_400),
        },
    )
    .await
    .map_err(AppError::from)?;
    Ok(())
}

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &axum::http::HeaderMap,
    method: &axum::http::Method,
    uri: &axum::http::Uri,
    request_id: String,
    required_scope: &str,
) -> Result<PublicApiContext, AppError> {
    metrics::counter!("drive_api_key_legacy_requests_total").increment(1);
    let credential = match http_signatures::signature_identity(headers)? {
        Some(identity) => PublicApiCredential::HttpSignature(identity.key_id),
        None => PublicApiCredential::Bearer(bearer_token(headers)?),
    };
    let hash = match &credential {
        PublicApiCredential::Bearer(token) | PublicApiCredential::HttpSignature(token) => {
            key_hash(token)
        }
    };
    let row_opt = db::get_api_key_by_hash(db, &hash).await?;

    if let Some(row) = row_opt {
        return api_key::authenticate_api_key(
            db,
            redis,
            headers,
            method,
            uri,
            request_id,
            required_scope,
            credential,
            row,
        )
        .await;
    }

    if let PublicApiCredential::Bearer(token) = &credential {
        return m2m::authenticate_m2m(db, redis, headers, token, request_id, required_scope).await;
    }

    Err(PublicApiErrorKind::InvalidApiKey.app_error())
}
