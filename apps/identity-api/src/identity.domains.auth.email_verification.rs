use chrono::{DateTime, Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    generate_random_token, log_dev_token, normalize_email, token_hash, types::*, validate_email,
};
use crate::http::error::AppError;

const CLEANUP_ADVISORY_LOCK_ID: i64 = 20260519;

pub fn verification_resend_available_at(
    sent_at: DateTime<Utc>,
    cooldown_seconds: i64,
) -> DateTime<Utc> {
    sent_at + ChronoDuration::seconds(cooldown_seconds)
}

pub async fn issue_verification_email_tx(
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Uuid,
    _email: &str,
    _display_name: &str,
    verification_token: &str,
) -> Result<DateTime<Utc>, AppError> {
    let now = Utc::now();
    let expires_at = now + ChronoDuration::hours(config.auth_verification_ttl_hours);
    nvbes_redis::email_verification::store_email_verification_token(
        redis,
        &nvbes_redis::email_verification::CachedEmailVerificationToken {
            principal_id,
            token_hash: token_hash(verification_token),
            created_at: now,
            expires_at,
            consumed_at: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("email_verification_token_store_failed", &err.to_string()))?;

    Ok(now)
}

pub async fn resend_verification_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: &str,
) -> Result<ResendVerificationResult, AppError> {
    let email = normalize_email(email);
    validate_email(&email)?;

    let row = sqlx::query(
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
        "#,
    )
    .bind(&email)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(ResendVerificationResult {
            success: true,
            email_verified: false,
            verification_resend_available_at: None,
        });
    };

    let principal_id: Uuid = row.get("principal_id");
    let email_verified_at: Option<DateTime<Utc>> = row.get("email_verified_at");
    if email_verified_at.is_some() {
        return Ok(ResendVerificationResult {
            success: true,
            email_verified: true,
            verification_resend_available_at: None,
        });
    }

    let last_token =
        nvbes_redis::email_verification::latest_unconsumed_email_verification_token_for_principal(
            redis,
            principal_id,
        )
        .await
        .map_err(|err| {
            AppError::internal("email_verification_token_read_failed", &err.to_string())
        })?;

    let now = Utc::now();
    let resend_available_at = last_token
        .as_ref()
        .map(|token| {
            verification_resend_available_at(
                token.created_at,
                config.auth_verification_resend_cooldown_seconds,
            )
        })
        .unwrap_or(now);

    if now < resend_available_at {
        return Ok(ResendVerificationResult {
            success: true,
            email_verified: false,
            verification_resend_available_at: Some(resend_available_at),
        });
    }

    nvbes_redis::email_verification::consume_all_email_verification_tokens_for_principal(
        redis,
        principal_id,
    )
    .await
    .map_err(|err| {
        AppError::internal("email_verification_token_consume_failed", &err.to_string())
    })?;

    let firstname: Option<String> = row.get("firstname");
    let lastname: Option<String> = row.get("lastname");
    let username: Option<String> = row.get("username");
    let display_name = crate::domains::auth::types::derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );
    let verification_token = generate_random_token();
    let verification_created_at = issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;
    let email_msg = crate::email::templates::verification_email(
        config,
        &email,
        &display_name,
        &verification_token,
    )?;
    crate::email::jobs::enqueue_email_job_tx(
        db,
        redis,
        crate::email::jobs::EmailSendPayload {
            to_email: email.to_string(),
            to_name: Some(display_name.to_string()),
            subject: email_msg.subject,
            html_body: email_msg.html_body.unwrap_or_default(),
            text_body: email_msg.text_body,
            business_type: "verification".to_string(),
        },
        &format!(
            "verify:{}:{}",
            principal_id,
            token_hash(&verification_token)
        ),
    )
    .await?;
    log_dev_token(
        &verification_token,
        &config.environment,
        "email_verification",
    );

    Ok(ResendVerificationResult {
        success: true,
        email_verified: false,
        verification_resend_available_at: Some(verification_resend_available_at(
            verification_created_at,
            config.auth_verification_resend_cooldown_seconds,
        )),
    })
}

pub async fn cleanup_expired_unverified_accounts(
    db: &PgPool,
    ttl_days: i64,
) -> Result<u64, AppError> {
    let mut tx = db.begin().await?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(CLEANUP_ADVISORY_LOCK_ID)
        .fetch_one(&mut *tx)
        .await?;

    if !locked {
        tx.rollback().await?;
        return Ok(0);
    }

    let deleted = sqlx::query(
        r#"
        DELETE FROM principals p
        USING users u
        WHERE p.id = u.principal_id
          AND u.status = 'pending_verification'
          AND u.email_verified_at IS NULL
           AND u.created_at < NOW() - make_interval(days => $1::int)
        RETURNING p.id
        "#,
    )
    .bind(ttl_days)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(deleted.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::verification_resend_available_at;
    use chrono::{TimeZone, Utc};

    #[test]
    fn resend_cooldown_is_added_to_sent_timestamp() {
        let sent_at = Utc.with_ymd_and_hms(2026, 5, 19, 13, 0, 0).unwrap();
        let available_at = verification_resend_available_at(sent_at, 45);
        assert_eq!(
            available_at,
            Utc.with_ymd_and_hms(2026, 5, 19, 13, 0, 45).unwrap()
        );
    }
}
