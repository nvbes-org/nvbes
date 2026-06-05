use crate::http::error::AppError;
use axum::http::HeaderMap;
use sqlx::PgPool;

use super::core;
use super::security;
pub use super::types::*;

pub async fn authenticate(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<AuthContext, AppError> {
    core::authenticate(db, token, request_headers).await
}

pub async fn logout(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<LogoutResult, AppError> {
    let _ = authenticate(db, token, request_headers).await?;
    Ok(LogoutResult {
        message: "Logged out.",
    })
}

pub async fn get_me(db: &PgPool, auth: &AuthContext) -> Result<MeResult, AppError> {
    core::get_me(db, auth).await
}

pub async fn me(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<MeResult, AppError> {
    let auth = authenticate(db, token, request_headers).await?;
    get_me(db, &auth).await
}

pub async fn require_recent_step_up(db: &PgPool, auth: &AuthContext) -> Result<(), AppError> {
    security::require_recent_step_up(db, auth).await
}
