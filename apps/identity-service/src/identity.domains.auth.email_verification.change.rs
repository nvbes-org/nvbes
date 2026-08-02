use super::{
    db, email_verification::issue_verification_email_tx,
    email_verification::resend_verification_email,
    email_verification::verification_resend_available_at, generate_random_token, normalize_email,
    registration_enrollment, types::*, validate_email,
};
use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};

pub async fn change_verification_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    registration_enrollment_token: &str,
    email: &str,
) -> Result<ResendVerificationResult, AppError> {
    let email = normalize_email(email);
    validate_email(&email)?;

    let mut tx = db.begin().await?;
    let principal_id =
        registration_enrollment::consume_tx(&mut tx, registration_enrollment_token).await?;

    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          u.email,
          COALESCE(profile.display_name, 'User') AS display_name,
          u.email_verified_at
        FROM users u
        LEFT JOIN identity_oidc_profile_claims profile
          ON profile.principal_id = u.principal_id
        WHERE u.principal_id = $1
        FOR UPDATE OF u
        "#,
    )
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;

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

    db::emails::change_unverified_primary_email(&mut tx, principal_id, &email).await?;
    registration_enrollment::consume_all_for_principal_tx(&mut tx, principal_id).await?;

    let display_name: String = row.get("display_name");
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
    let verification = issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;
    super::email_verification::enqueue_verification_email(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
        verification.expires_at,
    )
    .await?;
    Ok(ResendVerificationResult {
        success: true,
        email_verified: false,
        verification_resend_available_at: Some(verification_resend_available_at(
            verification.created_at,
            config.auth_verification_resend_cooldown_seconds,
        )),
    })
}
