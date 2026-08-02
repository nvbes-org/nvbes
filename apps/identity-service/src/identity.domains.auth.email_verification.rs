use chrono::{DateTime, Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{generate_random_token, normalize_email, token_hash, types::*, validate_email};
use crate::http::error::AppError;

const CLEANUP_ADVISORY_LOCK_ID: i64 = 20260519;

#[derive(Debug, Clone, Copy)]
pub struct VerificationIssue {
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

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
) -> Result<VerificationIssue, AppError> {
    let now = Utc::now();
    let expires_at = now + ChronoDuration::hours(config.auth_verification_ttl_hours);
    nvbes_redis::email_verification::store_email_verification_token(
        redis,
        &nvbes_redis::email_verification::CachedEmailVerificationToken {
            principal_id,
            email_address_id: None,
            email: None,
            purpose: "primary_email".to_string(),
            token_hash: token_hash(verification_token),
            created_at: now,
            expires_at,
            consumed_at: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("email_verification_token_store_failed", err.to_string()))?;

    Ok(VerificationIssue {
        created_at: now,
        expires_at,
    })
}

pub async fn enqueue_verification_email(
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Uuid,
    email: &str,
    display_name: &str,
    verification_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    crate::email::commands::enqueue(
        redis,
        email.to_string(),
        Some(display_name.to_string()),
        format!("verify:{principal_id}:{}", token_hash(verification_token)),
        nvbes_email::EmailTemplate::EmailVerificationV1 {
            user_name: display_name.to_string(),
            verification_url: crate::email::commands::verification_url(config, verification_token),
            credential_expires_at: expires_at,
        },
        expires_at,
        Some(principal_id),
    )
    .await
}

pub async fn resend_verification_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: &str,
) -> Result<ResendVerificationResult, AppError> {
    let email = normalize_email(email);
    validate_email(&email)?;
    let public_resend_available_at =
        Utc::now() + ChronoDuration::seconds(config.auth_verification_resend_cooldown_seconds);

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
        WHERE lower(u.email) = lower($1)
        "#,
    )
    .bind(&email)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(opaque_resend_result(public_resend_available_at));
    };

    let principal_id: Uuid = row.get("principal_id");
    let email_verified_at: Option<DateTime<Utc>> = row.get("email_verified_at");
    if email_verified_at.is_some() {
        return Ok(opaque_resend_result(public_resend_available_at));
    }

    let last_token =
        nvbes_redis::email_verification::latest_unconsumed_email_verification_token_for_principal(
            redis,
            principal_id,
        )
        .await
        .map_err(|err| {
            AppError::internal("email_verification_token_read_failed", err.to_string())
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
        return Ok(opaque_resend_result(public_resend_available_at));
    }

    nvbes_redis::email_verification::consume_all_email_verification_tokens_for_principal(
        redis,
        principal_id,
    )
    .await
    .map_err(|err| {
        AppError::internal("email_verification_token_consume_failed", err.to_string())
    })?;

    let display_name: String = row.get("display_name");
    let verification_token = generate_random_token();
    let issue = issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;
    enqueue_verification_email(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
        issue.expires_at,
    )
    .await?;
    Ok(opaque_resend_result(public_resend_available_at))
}

/// Keep the public response independent from account existence and verification state.
fn opaque_resend_result(
    verification_resend_available_at: DateTime<Utc>,
) -> ResendVerificationResult {
    ResendVerificationResult {
        success: true,
        email_verified: false,
        verification_resend_available_at: Some(verification_resend_available_at),
    }
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
    use super::{opaque_resend_result, verification_resend_available_at};
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

    #[test]
    fn public_resend_result_does_not_reveal_account_state() {
        let available_at = Utc.with_ymd_and_hms(2026, 7, 27, 13, 0, 45).unwrap();
        let result = opaque_resend_result(available_at);

        assert!(result.success);
        assert!(!result.email_verified);
        assert_eq!(result.verification_resend_available_at, Some(available_at));
    }
}
