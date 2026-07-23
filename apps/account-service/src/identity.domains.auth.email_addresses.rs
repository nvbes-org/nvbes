use chrono::{DateTime, Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    email_verification::verification_resend_available_at,
    generate_random_token, token_hash,
    types::{
        AddSecondaryEmailInput, AddSecondaryEmailResult, DeleteSecondaryEmailResult,
        EmailAddressesResult, PromoteSecondaryEmailResult, ResendSecondaryEmailVerificationResult,
    },
};
use crate::http::error::AppError;

pub async fn list(
    db: &PgPool,
    principal_id: Uuid,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<EmailAddressesResult, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    let cursor = cursor
        .as_deref()
        .map(nvbes_core::pagination::decode_cursor::<nvbes_core::pagination::KeysetCursor>)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let emails =
        db::emails::list_email_addresses(db, principal_id, cursor.as_ref(), (limit + 1) as i64)
            .await?;
    let page = nvbes_core::pagination::page_from_rows(emails, limit, |email| {
        nvbes_core::pagination::KeysetCursor {
            created_at: email.created_at,
            id: email.id,
        }
    });
    let next_cursor = page
        .next_cursor
        .map(|c| nvbes_core::pagination::encode_cursor(&c))
        .transpose()
        .map_err(|_| {
            AppError::internal("pagination_error", "Failed to encode pagination cursor.")
        })?;
    Ok(EmailAddressesResult {
        emails: page.items,
        primary_min_age_hours: db::emails::primary_email_policy_hours(db, principal_id).await?,
        next_cursor,
        has_more: page.has_more,
    })
}

pub async fn add_secondary(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Uuid,
    input: AddSecondaryEmailInput,
) -> Result<AddSecondaryEmailResult, AppError> {
    crate::email::templates::ensure_delivery_configured(config)?;

    let verification_token = generate_random_token();
    let mut tx = db.begin().await?;
    let email = db::emails::insert_secondary_email(&mut tx, principal_id, &input.email).await?;
    tx.commit().await?;

    let verification_created_at = issue_secondary_verification_token(
        redis,
        config,
        principal_id,
        email.id,
        &email.email,
        &verification_token,
    )
    .await?;

    enqueue_secondary_verification_email(db, redis, config, &email.email, &verification_token)
        .await?;
    notify_email_added(db, redis, principal_id, &email.email).await?;
    Ok(AddSecondaryEmailResult {
        email,
        verification_resend_available_at: verification_resend_available_at(
            verification_created_at,
            config.auth_verification_resend_cooldown_seconds,
        ),
    })
}

pub async fn resend_secondary_verification(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<ResendSecondaryEmailVerificationResult, AppError> {
    crate::email::templates::ensure_delivery_configured(config)?;

    let mut tx = db.begin().await?;
    let email =
        db::emails::fetch_email_address_for_update(&mut tx, principal_id, email_address_id).await?;
    if email.is_primary {
        return Err(AppError::bad_request(
            "primary_email_verification_not_supported",
            "Use the primary email verification flow for the primary email.",
        ));
    }
    if email.verified {
        return Err(AppError::bad_request(
            "email_already_verified",
            "This email is already verified.",
        ));
    }
    tx.commit().await?;

    let verification_token = generate_random_token();
    let verification_created_at = issue_secondary_verification_token(
        redis,
        config,
        principal_id,
        email.id,
        &email.email,
        &verification_token,
    )
    .await?;
    enqueue_secondary_verification_email(db, redis, config, &email.email, &verification_token)
        .await?;
    Ok(ResendSecondaryEmailVerificationResult {
        email,
        verification_resend_available_at: verification_resend_available_at(
            verification_created_at,
            config.auth_verification_resend_cooldown_seconds,
        ),
    })
}

pub async fn mark_secondary_verified(
    db: &PgPool,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<(), AppError> {
    db::emails::mark_secondary_verified(db, principal_id, email_address_id).await
}

pub async fn promote_secondary(
    db: &PgPool,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<PromoteSecondaryEmailResult, AppError> {
    let min_age_hours = db::emails::primary_email_policy_hours(db, principal_id).await?;
    let mut tx = db.begin().await?;
    let email =
        db::emails::promote_secondary_email(&mut tx, principal_id, email_address_id, min_age_hours)
            .await?;
    tx.commit().await?;
    let user = db::fetch_user_view(db, principal_id).await?;
    Ok(PromoteSecondaryEmailResult { email, user })
}

pub async fn delete_secondary(
    db: &PgPool,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<DeleteSecondaryEmailResult, AppError> {
    let mut tx = db.begin().await?;
    db::emails::delete_secondary_email(&mut tx, principal_id, email_address_id).await?;
    tx.commit().await?;
    Ok(DeleteSecondaryEmailResult { success: true })
}

async fn issue_secondary_verification_token(
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    principal_id: Uuid,
    email_address_id: Uuid,
    email: &str,
    verification_token: &str,
) -> Result<DateTime<Utc>, AppError> {
    let now = Utc::now();
    let expires_at = now + ChronoDuration::hours(config.auth_verification_ttl_hours);
    nvbes_redis::email_verification::store_email_verification_token(
        redis,
        &nvbes_redis::email_verification::CachedEmailVerificationToken {
            principal_id,
            email_address_id: Some(email_address_id),
            email: Some(email.to_string()),
            purpose: "secondary_email".to_string(),
            token_hash: token_hash(verification_token),
            created_at: now,
            expires_at,
            consumed_at: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("email_verification_token_store_failed", err.to_string()))?;
    Ok(now)
}

async fn enqueue_secondary_verification_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: &str,
    token: &str,
) -> Result<(), AppError> {
    let message = crate::email::templates::verification_email(config, email, email, token)?;
    crate::email::jobs::enqueue_email_job_tx(
        db,
        redis,
        crate::email::jobs::EmailSendPayload {
            to_email: email.to_string(),
            to_name: None,
            subject: message.subject,
            html_body: message.html_body.unwrap_or_default(),
            text_body: message.text_body,
            business_type: "verification".to_string(),
        },
        &format!("secondary-verify:{}:{}", email, token_hash(token)),
    )
    .await
    .map_err(AppError::from)
}

async fn notify_email_added(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    added_email: &str,
) -> Result<(), AppError> {
    let recipients = db::emails::active_email_recipients(db, principal_id).await?;
    for recipient in recipients {
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            crate::email::jobs::EmailSendPayload {
                to_email: recipient.clone(),
                to_name: None,
                subject: "New email added to your nvbes account".to_string(),
                html_body: format!(
                    "<p>A secondary email address was added to your nvbes account.</p><p><strong>{}</strong></p><p>If this was not you, secure your account immediately.</p>",
                    crate::email::templates::html_escape(added_email)
                ),
                text_body: Some(format!(
                    "A secondary email address was added to your nvbes account: {added_email}\n\nIf this was not you, secure your account immediately."
                )),
                business_type: "account_security".to_string(),
            },
            &format!("email-added:{}:{}:{}", principal_id, recipient, added_email),
        )
        .await?;
    }
    Ok(())
}
