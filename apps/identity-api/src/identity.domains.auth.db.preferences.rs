use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::types::*;
use crate::http::error::AppError;

pub async fn fetch_user_preferences(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<UserPreferences, AppError> {
    let row = sqlx::query("SELECT preferences FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .fetch_optional(db)
        .await?;

    let prefs: serde_json::Value = row
        .as_ref()
        .and_then(|value| sqlx::Row::try_get::<serde_json::Value, _>(value, "preferences").ok())
        .unwrap_or(serde_json::json!({}));

    Ok(UserPreferences {
        theme: prefs["theme"].as_str().unwrap_or("system").to_string(),
        language: prefs["language"].as_str().unwrap_or("en").to_string(),
        skip_password: prefs["skip_password"].as_bool().unwrap_or(false),
    })
}

pub async fn update_user_preferences(
    db: &PgPool,
    principal_id: Uuid,
    input: &UserPreferences,
) -> Result<UserPreferences, AppError> {
    let json_value = serde_json::json!({
        "theme": input.theme,
        "language": input.language,
        "skip_password": input.skip_password,
    });

    sqlx::query("UPDATE users SET preferences = $2, updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .bind(&json_value)
        .execute(db)
        .await?;

    Ok(input.clone())
}

pub async fn fetch_user_notifications(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<UserNotifications, AppError> {
    let row = sqlx::query("SELECT notifications FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .fetch_optional(db)
        .await?;

    let notifications: serde_json::Value = row
        .as_ref()
        .and_then(|value| sqlx::Row::try_get::<serde_json::Value, _>(value, "notifications").ok())
        .unwrap_or(serde_json::json!({}));

    Ok(UserNotifications {
        email: notifications["email"].as_bool().unwrap_or(true),
        push: notifications["push"].as_bool().unwrap_or(true),
        in_app: notifications["in_app"].as_bool().unwrap_or(true),
    })
}

pub async fn update_user_notifications(
    db: &PgPool,
    principal_id: Uuid,
    input: &UserNotifications,
) -> Result<UserNotifications, AppError> {
    let json_value = serde_json::json!({
        "email": input.email,
        "push": input.push,
        "in_app": input.in_app,
    });

    sqlx::query("UPDATE users SET notifications = $2, updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .bind(&json_value)
        .execute(db)
        .await?;

    Ok(input.clone())
}
