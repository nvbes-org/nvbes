use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use nvbes_core::auth::hash_password;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::auth::authenticate;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub principal_id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub session_token: String,
    pub principal_id: Uuid,
    pub expires_in_hours: i64,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .with_state(state.clone())
}

async fn register(
    State(state): State<IdentityState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), HttpError> {
    let email = normalize_email(&req.email)?;
    validate_password(&req.password)?;
    let password_hash = hash_password(&req.password)
        .map_err(|_| HttpError::BadRequest("Password hashing failed".to_string()))?;

    let principal_id = Uuid::new_v4();
    let mut tx = state.db.begin().await
        .map_err(|e| HttpError::InternalServerError(format!("Database transaction failed: {}", e)))?;

    sqlx::query(
        "INSERT INTO identity_principals (id, kind, status) VALUES ($1, 'human', 'active')",
    )
    .bind(principal_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| HttpError::InternalServerError(format!("Failed to create principal: {}", e)))?;

    sqlx::query(
        "INSERT INTO identity_login_identifiers (id, principal_id, kind, normalized_value, verified_at) VALUES ($1, $2, 'email', $3, clock_timestamp())",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(&email)
    .execute(&mut *tx)
    .await
    .map_err(|e| HttpError::InternalServerError(format!("Failed to create login identifier: {}", e)))?;

    sqlx::query(
        "INSERT INTO identity_password_credentials (principal_id, password_hash) VALUES ($1, $2)",
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| HttpError::InternalServerError(format!("Failed to create password credential: {}", e)))?;

    audit(&mut tx, principal_id, "identity.registered").await
        .map_err(|e| HttpError::InternalServerError(format!("Failed to audit registration: {}", e)))?;

    tx.commit().await
        .map_err(|e| HttpError::InternalServerError(format!("Failed to commit registration: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            principal_id,
            message: "Registration successful".to_string(),
        }),
    ))
}

async fn login(
    State(state): State<IdentityState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), HttpError> {
    let session_token = authenticate(&state.db, &req.email, &req.password)
        .await
        .map_err(|e| HttpError::Unauthorized(e.to_string()))?;

    let principal_id: Uuid = sqlx::query_scalar(
        "SELECT p.id FROM identity_principals p 
         JOIN identity_login_identifiers i ON i.principal_id = p.id 
         WHERE i.kind = 'email' AND i.normalized_value = $1",
    )
    .bind(req.email.to_ascii_lowercase().trim())
    .fetch_one(&state.db)
    .await
    .map_err(|e| HttpError::InternalServerError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            session_token,
            principal_id,
            expires_in_hours: 1,
        }),
    ))
}

#[derive(Debug)]
enum HttpError {
    BadRequest(String),
    Unauthorized(String),
    InternalServerError(String),
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            HttpError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            HttpError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            HttpError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

fn normalize_email(email: &str) -> Result<String, HttpError> {
    let email = email.trim().to_ascii_lowercase();
    let valid = email.len() <= 320
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    valid
        .then_some(email)
        .ok_or_else(|| HttpError::BadRequest("Invalid email address".to_string()))
}

fn validate_password(password: &str) -> Result<(), HttpError> {
    if !(12..=1024).contains(&password.len()) {
        return Err(HttpError::BadRequest("Password must be between 12 and 1024 characters".to_string()));
    }
    Ok(())
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    event_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO identity_audit_events (id, principal_id, actor_principal_id, event_type, correlation_id) VALUES ($1, $2, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(principal_id)
        .bind(event_type)
        .bind(Uuid::new_v4())
        .execute(&mut **tx)
        .await?;
    Ok(())
}