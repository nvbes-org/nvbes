use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

pub(super) async fn notify_email_added(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    added_email: &str,
) -> Result<(), AppError> {
    for recipient in
        super::db::emails::verified_security_notification_recipients(db, principal_id).await?
    {
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            email_added_payload(&recipient, added_email),
            &format!("email-added:{principal_id}:{recipient}:{added_email}"),
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
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            primary_email_changed_payload(&recipient, previous_primary_email, new_primary_email),
            &format!("primary-email-changed:{principal_id}:{recipient}:{new_primary_email}"),
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
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            account_recovered_payload(&recipient),
            &format!("account-recovered:{principal_id}:{recipient}:{recovery_event_id}"),
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
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            recovery_review_payload(&recipient),
            &format!("recovery-review:{request_id}:{recipient}"),
        )
        .await?;
    }
    Ok(())
}

fn email_added_payload(recipient: &str, added_email: &str) -> crate::email::jobs::EmailSendPayload {
    crate::email::jobs::EmailSendPayload {
        to_email: recipient.to_string(),
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
    }
}

fn primary_email_changed_payload(
    recipient: &str,
    previous_primary_email: &str,
    new_primary_email: &str,
) -> crate::email::jobs::EmailSendPayload {
    crate::email::jobs::EmailSendPayload {
        to_email: recipient.to_string(),
        to_name: None,
        subject: "Primary email changed on your nvbes account".to_string(),
        html_body: format!(
            "<p>The primary email address on your nvbes account changed.</p><p>Previous: <strong>{}</strong></p><p>New: <strong>{}</strong></p><p>If this was not you, secure your account immediately.</p>",
            crate::email::templates::html_escape(previous_primary_email),
            crate::email::templates::html_escape(new_primary_email),
        ),
        text_body: Some(format!(
            "The primary email address on your nvbes account changed.\n\nPrevious: {previous_primary_email}\nNew: {new_primary_email}\n\nIf this was not you, secure your account immediately."
        )),
        business_type: "account_security".to_string(),
    }
}

fn account_recovered_payload(recipient: &str) -> crate::email::jobs::EmailSendPayload {
    crate::email::jobs::EmailSendPayload {
        to_email: recipient.to_string(),
        to_name: None,
        subject: "Your nvbes account was recovered".to_string(),
        html_body: "<p>Your nvbes password was reset and all existing sessions, refresh tokens, and trusted devices were revoked.</p><p>If you did not perform this recovery, contact support immediately.</p>".to_string(),
        text_body: Some(
            "Your nvbes password was reset and all existing sessions, refresh tokens, and trusted devices were revoked.\n\nIf you did not perform this recovery, contact support immediately."
                .to_string(),
        ),
        business_type: "account_security".to_string(),
    }
}

fn recovery_review_payload(recipient: &str) -> crate::email::jobs::EmailSendPayload {
    crate::email::jobs::EmailSendPayload {
        to_email: recipient.to_string(),
        to_name: None,
        subject: "Account recovery requires security review".to_string(),
        html_body: "<p>A recovery attempt for your nvbes account was classified as high risk and is awaiting a controlled security review.</p><p>No password-reset token has been issued. If this was not you, secure your account immediately.</p>".to_string(),
        text_body: Some(
            "A recovery attempt for your nvbes account was classified as high risk and is awaiting a controlled security review.\n\nNo password-reset token has been issued. If this was not you, secure your account immediately."
                .to_string(),
        ),
        business_type: "account_security".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::primary_email_changed_payload;

    #[test]
    fn primary_email_change_notification_escapes_html_and_targets_recipient() {
        let payload = primary_email_changed_payload(
            "previous@example.com",
            "previous+<alert>@example.com",
            "new+<script>@example.com",
        );

        assert_eq!(payload.to_email, "previous@example.com");
        assert_eq!(payload.business_type, "account_security");
        assert!(!payload.html_body.contains("<alert>"));
        assert!(!payload.html_body.contains("<script>"));
        assert!(payload.html_body.contains("&lt;alert&gt;"));
        assert!(payload.html_body.contains("&lt;script&gt;"));
        assert!(
            payload
                .text_body
                .as_deref()
                .is_some_and(|body| body.contains("previous+<alert>@example.com"))
        );
    }
}
