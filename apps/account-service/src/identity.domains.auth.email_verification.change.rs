use chrono::{DateTime, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    email_verification::issue_verification_email_tx, email_verification::resend_verification_email,
    email_verification::verification_resend_available_at, generate_random_token, normalize_email,
    types::*, validate_email,
};
use crate::http::error::AppError;

pub async fn change_verification_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Option<Uuid>,
    current_email: Option<&str>,
    email: &str,
) -> Result<ResendVerificationResult, AppError> {
    let email = normalize_email(email);
    validate_email(&email)?;

    let mut tx = db.begin().await?;

    let row = if let Some(principal_id) = principal_id {
        sqlx::query(
            r#"
            SELECT
              u.principal_id,
              u.email,
              u.firstname,
              u.lastname,
              u.username,
              u.email_verified_at
            FROM users u
            WHERE u.principal_id = $1
            FOR UPDATE
            "#,
        )
        .bind(principal_id)
        .fetch_one(&mut *tx)
        .await?
    } else {
        let current_email = current_email.map(normalize_email).ok_or_else(|| {
            AppError::bad_request("current_email_required", "Current email is required.")
        })?;

        sqlx::query(
            r#"
            SELECT
              u.principal_id,
              u.email,
              u.firstname,
              u.lastname,
              u.username,
              u.email_verified_at
            FROM users u
            WHERE lower(u.email) = lower($1)
            FOR UPDATE
            "#,
        )
        .bind(current_email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AppError::not_found("verification_account_not_found", "Account not found.")
        })?
    };

    let principal_id: Uuid = row.get("principal_id");
    let email_verified_at: Option<DateTime<Utc>> = row.get("email_verified_at");
    if email_verified_at.is_some() {
        tx.rollback().await?;
        return Err(AppError::forbidden(
            "email_already_verified",
            "This account is already verified.",
        ));
    }

    let current_email = row.get::<String, _>("email");
    if current_email.trim().eq_ignore_ascii_case(&email) {
        tx.rollback().await?;
        return resend_verification_email(db, redis, config, &email).await;
    }

    let email_conflict = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(1)
        FROM users
        WHERE lower(email) = lower($1)
          AND principal_id <> $2
        "#,
    )
    .bind(&email)
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;

    if email_conflict > 0 {
        tx.rollback().await?;
        return Err(AppError::conflict(
            "email_already_exists",
            "This email is already associated with another account.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE users
        SET email = $2,
            email_verified_at = NULL,
            status = 'pending_verification',
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .execute(&mut *tx)
    .await?;

    let firstname: Option<String> = row.get("firstname");
    let lastname: Option<String> = row.get("lastname");
    let username: Option<String> = row.get("username");
    let display_name = crate::domains::auth::types::derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );
    let verification_token = generate_random_token();

    tx.commit().await?;
    nvbes_redis::email_verification::consume_all_email_verification_tokens_for_principal(
        redis,
        principal_id,
    )
    .await
    .map_err(|err| {
        AppError::internal("email_verification_token_consume_failed", err.to_string())
    })?;
    let verification_created_at = issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;
    Ok(ResendVerificationResult {
        success: true,
        email_verified: false,
        verification_resend_available_at: Some(verification_resend_available_at(
            verification_created_at,
            config.auth_verification_resend_cooldown_seconds,
        )),
    })
}
