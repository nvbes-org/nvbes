#[path = "drive.domains.auth.core.rs"]
pub mod core;
#[path = "drive.domains.auth.identity.rs"]
pub mod identity;
#[path = "drive.domains.auth.identity_sync.rs"]
pub mod identity_sync;
#[path = "drive.domains.auth.jwt.rs"]
pub mod jwt;
#[path = "drive.domains.auth.security.rs"]
pub mod security;

#[path = "drive.domains.auth.current.rs"]
pub mod current;
#[path = "drive.domains.auth.service.rs"]
pub mod service;
#[path = "drive.domains.auth.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "drive.domains.auth.e2e.tests.rs"]
mod e2e_tests;

use axum::Router;
use axum::http::HeaderMap;
use sqlx::PgPool;

pub async fn authenticate(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<types::AuthContext, crate::http::error::AppError> {
    service::authenticate(db, token, request_headers).await
}

pub async fn logout(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<types::LogoutResult, crate::http::error::AppError> {
    service::logout(db, token, request_headers).await
}

pub async fn me(
    db: &PgPool,
    token: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<types::MeResult, crate::http::error::AppError> {
    service::me(db, token, request_headers).await
}

pub async fn require_recent_step_up(
    db: &PgPool,
    auth: &types::AuthContext,
) -> Result<(), crate::http::error::AppError> {
    service::require_recent_step_up(db, auth).await
}

pub fn router(state: &crate::app::AppState) -> Router<crate::app::AppState> {
    current::router(state)
}
