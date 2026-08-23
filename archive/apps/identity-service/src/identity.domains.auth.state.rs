use chrono::{Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthState {
    pub id: Uuid,
    pub principal_id: Option<Uuid>,
    pub email: String,
    pub next_step: String,
    pub device_fingerprint: Option<Value>,
    pub completed_methods: Vec<String>,
}

pub async fn create_state(
    redis: &nvbes_redis::RedisPool,
    principal_id: Option<Uuid>,
    email: &str,
    next_step: &str,
    device_fingerprint: Option<Value>,
    completed_methods: Vec<String>,
) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(15);
    nvbes_redis::auth_state::set_auth_state(
        redis,
        &nvbes_redis::auth_state::CachedAuthState {
            id: id.to_string(),
            principal_id: principal_id.map(|value| value.to_string()),
            email: email.to_string(),
            next_step: next_step.to_string(),
            device_fingerprint,
            completed_methods,
            expires_at,
        },
        15 * 60,
    )
    .await
    .map_err(|err| AppError::internal("auth_state_store_failed", format!("{}", err)))?;

    Ok(id)
}

pub async fn consume_state(
    redis: &nvbes_redis::RedisPool,
    id: Uuid,
    expected_step: &str,
) -> Result<AuthState, AppError> {
    let state = fetch_state(redis, id, expected_step).await?;
    delete_state(redis, id).await?;
    Ok(state)
}

pub async fn fetch_state(
    redis: &nvbes_redis::RedisPool,
    id: Uuid,
    expected_step: &str,
) -> Result<AuthState, AppError> {
    let state = nvbes_redis::auth_state::get_auth_state(redis, &id.to_string())
        .await
        .map_err(|err| AppError::internal("auth_state_load_failed", format!("{}", err)))?
        .ok_or_else(|| {
            AppError::unauthorized(
                "invalid_auth_state",
                "The authentication session is invalid or has expired.",
            )
        })?;
    if state.expires_at <= Utc::now() {
        return Err(AppError::unauthorized(
            "invalid_auth_state",
            "The authentication session is invalid or has expired.",
        ));
    }

    if state.next_step != expected_step {
        return Err(AppError::unauthorized(
            "invalid_auth_step",
            "Invalid authentication step.",
        ));
    }

    Ok(AuthState {
        id,
        principal_id: state
            .principal_id
            .and_then(|value| Uuid::parse_str(&value).ok()),
        email: state.email,
        next_step: state.next_step,
        device_fingerprint: state.device_fingerprint,
        completed_methods: state.completed_methods,
    })
}

pub async fn delete_state(redis: &nvbes_redis::RedisPool, id: Uuid) -> Result<(), AppError> {
    nvbes_redis::auth_state::delete_auth_state(redis, &id.to_string())
        .await
        .map_err(|err| AppError::internal("auth_state_delete_failed", format!("{}", err)))?;
    Ok(())
}
