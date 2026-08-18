use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use nvbes_email::{AccountSecurityEvent, EmailTemplate};

use crate::{domains::auth::token_hash, http::error::AppError};

pub(super) async fn notify_email_added(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    added_email: &str,
) -> Result<(), AppError> {
    for recipient in
        super::db::emails::verified_security_notification_recipients(db, principal_id).await?
    {
        enqueue_security(
            redis,
            principal_id,
            &recipient,
            format!(
                "email-added:{principal_id}:{}:{}",
                token_hash(&recipient),
                token_hash(added_email)
            ),
            AccountSecurityEvent::EmailAdded,
            Some(added_email.to_string()),
            None,
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn notify_primary_email_changed(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    previous_primary_email: &str,
    new_primary_email: &str,
) -> Result<(), AppError> {
    let mut recipients =
        super::db::emails::verified_security_notification_recipients(db, principal_id).await?;
    recipients.push(previous_primary_email.to_string());
    recipients.push(new_primary_email.to_string());
    recipients.sort_by_key(|email| email.to_ascii_lowercase());
    recipients.dedup_by(|left, right| left.eq_ignore_ascii_case(right));

    for recipient in recipients {
        enqueue_security(
            redis,
            principal_id,
            &recipient,
            format!(
                "primary-email-changed:{principal_id}:{}:{}",
                token_hash(&recipient),
                token_hash(new_primary_email)
            ),
            AccountSecurityEvent::PrimaryEmailChanged,
            Some(new_primary_email.to_string()),
            Some(previous_primary_email.to_string()),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn notify_account_recovered(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    recovery_event_id: &str,
) -> Result<(), AppError> {
    for recipient in
        super::db::emails::verified_security_notification_recipients(db, principal_id).await?
    {
        enqueue_security(
            redis,
            principal_id,
            &recipient,
            format!(
                "account-recovered:{principal_id}:{}:{recovery_event_id}",
                token_hash(&recipient)
            ),
            AccountSecurityEvent::AccountRecovered,
            None,
            None,
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn notify_recovery_review_requested(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    request_id: Uuid,
) -> Result<(), AppError> {
    for recipient in
        super::db::emails::verified_security_notification_recipients(db, principal_id).await?
    {
        enqueue_security(
            redis,
            principal_id,
            &recipient,
            format!("recovery-review:{request_id}:{}", token_hash(&recipient)),
            AccountSecurityEvent::RecoveryReviewRequired,
            None,
            None,
        )
        .await?;
    }
    Ok(())
}

async fn enqueue_security(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    recipient: &str,
    idempotency_key: String,
    event: AccountSecurityEvent,
    affected_email: Option<String>,
    previous_email: Option<String>,
) -> Result<(), AppError> {
    let deliver_before = Utc::now() + Duration::hours(24);
    crate::email::commands::enqueue(
        redis,
        recipient.to_string(),
        None,
        idempotency_key,
        EmailTemplate::AccountSecurityV1 {
            event,
            affected_email,
            previous_email,
            security_url: None,
        },
        deliver_before,
        Some(principal_id),
    )
    .await
}
