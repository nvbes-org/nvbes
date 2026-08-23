use chrono::Utc;
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use super::password::history;
use super::{
    db, generate_random_token, hash_password_with_pepper, normalize_email, registration_enrollment,
    token_hash, types::*, validate_email, validate_password,
};
use crate::http::error::AppError;

pub async fn register(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: RegisterInput,
) -> Result<RegisterResult, AppError> {
    let email = normalize_email(&input.email);
    validate_email(&email)?;
    validate_password(&input.password)?;
    if !input.legal_documents_accepted {
        return Err(AppError::bad_request(
            "legal_documents_required",
            "Legal documents must be accepted to create an account.",
        ));
    }

    nvbes_core::limiter::check_rate_limit(
        redis,
        "register",
        &email,
        6,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let password_hash =
        hash_password_with_pepper(&input.password, config.auth_password_pepper.as_deref())?;
    let verification_token = generate_random_token();
    let registration_enrollment =
        registration_enrollment::generate(config.auth_verification_ttl_hours);
    let (principal_id, now, verification_expires_at) = db::create_user_account(
        db,
        redis,
        config,
        email.clone(),
        input.data_region.clone(),
        password_hash.clone(),
        verification_token.clone(),
        registration_enrollment.token_hash.clone(),
        registration_enrollment.expires_at,
        input.ip.clone(),
        input.user_agent.clone(),
        input.legal_documents_accepted,
        input.marketing_emails_accepted,
    )
    .await?;
    history::insert_password_hash(db, principal_id, &password_hash).await?;
    let display_name = DEFAULT_DISPLAY_NAME.to_string();
    let recipient_name = super::email_recipient::recipient_name(Some(&display_name));

    super::email_verification::enqueue_verification_email(
        redis,
        config,
        principal_id,
        &email,
        &recipient_name,
        &verification_token,
        verification_expires_at,
        &input.timezone,
    )
    .await?;

    Ok(RegisterResult {
        user: UserView {
            id: principal_id,
            email: email.clone(),
            display_name,
            email_verified: false,
            mfa_enabled: false,
            created_at: now,
        },
        verification_resend_available_at:
            super::email_verification::verification_resend_available_at(
                now,
                config.auth_verification_resend_cooldown_seconds,
            ),
        registration_enrollment_token: registration_enrollment.token,
    })
}

pub async fn verify_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    input: VerifyEmailInput,
) -> Result<VerifyEmailResult, AppError> {
    let token_hash = token_hash(&input.token);
    let row = nvbes_redis::email_verification::get_email_verification_token(redis, &token_hash)
        .await
        .map_err(|err| AppError::internal("email_verification_token_read_failed", err.to_string()))?
        .ok_or_else(|| {
            AppError::not_found(
                "verification_token_not_found",
                "Invalid verification token.",
            )
        })?;

    let principal_id: Uuid = row.principal_id;
    let expires_at: chrono::DateTime<Utc> = row.expires_at;
    let consumed_at: Option<chrono::DateTime<Utc>> = row.consumed_at;
    if consumed_at.is_some() || expires_at <= Utc::now() {
        return Err(AppError::forbidden(
            "verification_token_expired",
            "Verification token is expired or already used.",
        ));
    }

    nvbes_redis::email_verification::mark_email_verification_token_consumed(redis, &token_hash)
        .await
        .map_err(|err| {
            AppError::internal("email_verification_token_consume_failed", err.to_string())
        })?;

    if row.purpose == "secondary_email" {
        let email_address_id = row.email_address_id.ok_or_else(|| {
            AppError::bad_request(
                "verification_token_invalid",
                "Invalid secondary email verification token.",
            )
        })?;
        crate::domains::auth::email_addresses::mark_secondary_verified(
            db,
            principal_id,
            email_address_id,
        )
        .await?;
        let user = db::fetch_user_view(db, principal_id).await?;
        return Ok(VerifyEmailResult {
            success: true,
            user,
        });
    }

    let mut tx = db.begin().await?;
    sqlx::query("UPDATE users SET email_verified_at = NOW(), status = 'active', updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE user_email_addresses
        SET verified_at = COALESCE(verified_at, NOW()),
            updated_at = NOW()
        WHERE principal_id = $1
          AND is_primary = TRUE
          AND deleted_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE principals SET status = 'active', updated_at = NOW() WHERE id = $1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    registration_enrollment::delete_all_for_principal_tx(&mut tx, principal_id).await?;
    tx.commit().await?;

    Ok(VerifyEmailResult {
        success: true,
        user: db::fetch_user_view(db, principal_id).await?,
    })
}
