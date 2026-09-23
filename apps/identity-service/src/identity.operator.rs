use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::IdentityState,
    auth::{audit, hash_token},
    tokens::{OPERATOR_AUDIENCE, OPERATOR_ROLE, OperatorTokenClaims, TokenService},
    tokens_config::TokenConfig,
};

#[derive(Debug, Deserialize)]
pub struct IssueOperatorTokenRequest {
    pub session_token: String,
}

#[derive(Debug, Serialize)]
pub struct IssueOperatorTokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub role: &'static str,
    pub audience: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct RevokeSessionsRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct RevokeSessionsResponse {
    pub principal_id: Uuid,
    pub sessions_revoked: i64,
    pub refresh_tokens_revoked: i64,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/api/v1/operator/token", post(issue_token))
        .route(
            "/api/v1/operator/principals/{principal_id}/sessions/revoke",
            post(revoke_sessions),
        )
        .with_state(state.clone())
}

async fn issue_token(
    State(state): State<IdentityState>,
    Json(req): Json<IssueOperatorTokenRequest>,
) -> Result<Json<IssueOperatorTokenResponse>, OperatorError> {
    let now = Utc::now();
    let session: (Uuid, Uuid) = sqlx::query_as(
        "SELECT s.id, s.principal_id FROM identity_sessions s
         JOIN identity_principals p ON p.id = s.principal_id
         WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > $2
           AND s.step_up_expires_at IS NOT NULL AND s.step_up_expires_at > $2
           AND p.status = 'active'",
    )
    .bind(hash_token(&req.session_token))
    .bind(now)
    .fetch_optional(&state.db)
    .await?
    .ok_or(OperatorError::Unauthorized)?;
    let (_session_id, principal_id) = session;

    if !state
        .config
        .platform_operator_principals
        .contains(&principal_id)
    {
        return Err(OperatorError::Forbidden);
    }

    let has_totp: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id = $1 AND kind = 'totp' AND state = 'active')",
    )
    .bind(principal_id)
    .fetch_one(&state.db)
    .await?;
    if !has_totp {
        return Err(OperatorError::Forbidden);
    }

    let auth_time = now.timestamp();
    let token_service = token_service(&state)?;
    let access_token = token_service
        .issue_operator(principal_id, vec!["totp".into()], auth_time)
        .map_err(|_| OperatorError::Internal)?;

    let mut tx = state.db.begin().await?;
    audit(&mut tx, principal_id, "identity.operator_token_issued").await?;
    tx.commit().await?;

    Ok(Json(IssueOperatorTokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: 15 * 60,
        role: OPERATOR_ROLE,
        audience: OPERATOR_AUDIENCE,
    }))
}

async fn revoke_sessions(
    State(state): State<IdentityState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<RevokeSessionsRequest>,
) -> Result<Json<RevokeSessionsResponse>, OperatorError> {
    validate_reason(&req.reason)?;
    let operator = require_operator(&state, &headers)?;
    let operator_id = Uuid::parse_str(&operator.sub).map_err(|_| OperatorError::Unauthorized)?;

    let mut tx = state.db.begin().await?;
    let sessions_revoked: i64 = sqlx::query_scalar(
        "WITH updated AS (
            UPDATE identity_sessions SET revoked_at = clock_timestamp()
            WHERE principal_id = $1 AND revoked_at IS NULL
            RETURNING 1
         ) SELECT count(*) FROM updated",
    )
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;
    let refresh_tokens_revoked: i64 = sqlx::query_scalar(
        "WITH updated AS (
            UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp()
            WHERE principal_id = $1 AND revoked_at IS NULL
            RETURNING 1
         ) SELECT count(*) FROM updated",
    )
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO identity_audit_events (id, principal_id, actor_principal_id, event_type, correlation_id, details)
         VALUES ($1, $2, $3, 'identity.sessions_revoked_by_operator', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(operator_id)
    .bind(Uuid::new_v4())
    .bind(serde_json::json!({
        "reason": req.reason,
        "sessions_revoked": sessions_revoked,
        "refresh_tokens_revoked": refresh_tokens_revoked
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(RevokeSessionsResponse {
        principal_id,
        sessions_revoked,
        refresh_tokens_revoked,
    }))
}

fn require_operator(
    state: &IdentityState,
    headers: &HeaderMap,
) -> Result<OperatorTokenClaims, OperatorError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(OperatorError::Unauthorized)?;
    let claims = token_service(state)?
        .verify_operator(token)
        .map_err(|_| OperatorError::Unauthorized)?;
    let operator_id = Uuid::parse_str(&claims.sub).map_err(|_| OperatorError::Unauthorized)?;
    if !state
        .config
        .platform_operator_principals
        .contains(&operator_id)
    {
        return Err(OperatorError::Forbidden);
    }
    Ok(claims)
}

fn token_service(state: &IdentityState) -> Result<TokenService, OperatorError> {
    let config =
        TokenConfig::from_env(&state.config.environment).map_err(|_| OperatorError::Internal)?;
    TokenService::new(config).map_err(|_| OperatorError::Internal)
}

fn validate_reason(reason: &str) -> Result<(), OperatorError> {
    let trimmed = reason.trim();
    if !(12..=500).contains(&trimmed.len()) || reason.contains(['\r', '\n']) {
        return Err(OperatorError::Invalid("reason must be 12-500 characters"));
    }
    Ok(())
}

#[derive(Debug)]
enum OperatorError {
    Unauthorized,
    Forbidden,
    Invalid(&'static str),
    Database,
    Internal,
}

impl From<sqlx::Error> for OperatorError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

impl axum::response::IntoResponse for OperatorError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "operator authentication required",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "operator action forbidden",
            ),
            Self::Invalid(message) => (StatusCode::BAD_REQUEST, "invalid_request", message),
            Self::Database | Self::Internal => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "operator service unavailable",
            ),
        };
        (
            status,
            Json(serde_json::json!({"error": code, "message": message})),
        )
            .into_response()
    }
}

#[cfg(test)]
#[path = "identity.operator.tests.rs"]
mod tests;
