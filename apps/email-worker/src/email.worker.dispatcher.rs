use std::time::Duration;

use chrono::Duration as ChronoDuration;
use nvbes_email::{
    EmailAddress, EmailFailureClass, EmailMessage, EmailTemplate,
    proto::nvbes::email::v1::{EmailRecipient, TransactionalEmailTemplate},
};
use prost::Message;
use tokio::sync::watch;

use crate::{crypto::SealedValue, database, dispatch_db, state::EmailWorkerState};

pub async fn run(state: EmailWorkerState, mut shutdown: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                state.record_dispatcher_heartbeat();
                if let Err(error) = dispatch_db::suppress_due_messages(&state.db).await {
                    tracing::error!(error = ?error, "email suppression sweep failed");
                    continue;
                }
                match database::expire_stale_messages(&state.db).await {
                    Ok(expired) if expired > 0 => tracing::info!(expired, "expired stale email commands"),
                    Ok(_) => {}
                    Err(error) => tracing::error!(error = ?error, "email deadline sweep failed"),
                }
                if let Err(error) = dispatch_one(&state).await {
                    tracing::error!(error = ?error, "email dispatch iteration failed");
                }
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
}

async fn dispatch_one(state: &EmailWorkerState) -> anyhow::Result<()> {
    let Some(claim) = dispatch_db::claim_one(&state.db).await? else {
        return Ok(());
    };
    if dispatch_db::expire_claim_if_due(&state.db, &claim).await? {
        return Ok(());
    }

    let message = match materialize(state, &claim) {
        Ok(message) => message,
        Err(error) => {
            tracing::error!(message_id = %claim.id, error = ?error, "stored email command is invalid");
            dispatch_db::complete_failure(
                &state.db,
                &claim,
                "permanent_failure",
                "stored_command_invalid",
                1,
                ChronoDuration::zero(),
            )
            .await?;
            return Ok(());
        }
    };

    if dispatch_db::expire_claim_if_due(&state.db, &claim).await? {
        return Ok(());
    }
    match state
        .provider
        .send_message_before(&message, claim.deliver_before)
        .await
    {
        Ok(result) => {
            dispatch_db::complete_success(&state.db, &claim, &result.provider_email_id).await?
        }
        Err(error) => {
            let policy = retry_policy(&claim.category, claim.attempt_count);
            let outcome = match error.failure_class() {
                EmailFailureClass::Transient => "transient_failure",
                EmailFailureClass::Ambiguous => "ambiguous_failure",
                EmailFailureClass::Permanent => "permanent_failure",
            };
            tracing::warn!(
                message_id = %claim.id,
                error_code = error.safe_code(),
                error_class = ?error.failure_class(),
                attempt = claim.attempt_count,
                "email provider attempt failed"
            );
            dispatch_db::complete_failure(
                &state.db,
                &claim,
                outcome,
                error.safe_code(),
                policy.maximum_attempts,
                policy.retry_delay,
            )
            .await?;
        }
    }
    Ok(())
}

fn materialize(
    state: &EmailWorkerState,
    claim: &dispatch_db::ClaimedEmail,
) -> anyhow::Result<EmailMessage> {
    let recipient_bytes = state.crypto.open(
        claim.id,
        "recipient",
        &sealed(&claim.recipient_ciphertext, &claim.recipient_nonce)?,
    )?;
    let template_bytes = state.crypto.open(
        claim.id,
        "template",
        &sealed(&claim.template_ciphertext, &claim.template_nonce)?,
    )?;
    let recipient = EmailRecipient::decode(recipient_bytes.as_slice())?;
    let template = EmailTemplate::try_from(TransactionalEmailTemplate::decode(
        template_bytes.as_slice(),
    )?)?;
    let business_type = business_type(&template);
    let rendered = template.render();
    let mut headers = vec![
        ("Message-ID".to_string(), claim.message_id.clone()),
        ("X-Nvbes-Email-Message-Id".to_string(), claim.id.to_string()),
        ("X-Nvbes-Email-Job-Id".to_string(), claim.id.to_string()),
        (
            "X-Nvbes-Email-Business-Type".to_string(),
            business_type.to_string(),
        ),
    ];
    if let Some(reply_to) = state.config.reply_to.clone() {
        headers.push(("Reply-To".to_string(), reply_to));
    }
    Ok(EmailMessage {
        from: EmailAddress {
            email: state.config.from_email.clone(),
            name: Some(state.config.from_name.clone()),
        },
        to: vec![EmailAddress {
            email: recipient.email,
            name: recipient.name,
        }],
        subject: rendered.subject,
        text_body: Some(rendered.text_body),
        html_body: Some(rendered.html_body),
        headers,
    })
}

fn business_type(template: &EmailTemplate) -> &'static str {
    match template {
        EmailTemplate::EmailVerificationV1 { .. } => "verification",
        EmailTemplate::PasswordResetV1 { .. } => "password_reset",
        EmailTemplate::PasswordChangeCodeV1 { .. } => "password_change_code",
        EmailTemplate::AccountSecurityV1 { .. } => "account_security",
        EmailTemplate::BillingReceiptV1 { .. } => "billing_receipt",
        EmailTemplate::BillingPaymentFailureV1 { .. } => "billing_payment_failure",
        EmailTemplate::AccessReviewReminderV1 { .. } => "access_review_reminder",
    }
}

fn sealed(ciphertext: &[u8], nonce: &[u8]) -> anyhow::Result<SealedValue> {
    Ok(SealedValue {
        ciphertext: ciphertext.to_vec(),
        nonce: nonce
            .try_into()
            .map_err(|_| anyhow::anyhow!("stored email nonce is invalid"))?,
    })
}

struct RetryPolicy {
    maximum_attempts: i32,
    retry_delay: ChronoDuration,
}

fn retry_policy(category: &str, attempt: i32) -> RetryPolicy {
    let (maximum_attempts, delays): (i32, &[i64]) = match category {
        "credential" => (4, &[15, 60, 180]),
        "account_security" => (5, &[30, 120, 600, 1_800]),
        "billing" => (5, &[60, 300, 1_800, 7_200]),
        "reminder" => (4, &[60, 600, 3_600]),
        _ => (1, &[]),
    };
    let index = (attempt.saturating_sub(1) as usize).min(delays.len().saturating_sub(1));
    let base = delays.get(index).copied().unwrap_or(0);
    RetryPolicy {
        maximum_attempts,
        retry_delay: ChronoDuration::seconds(jitter(base, attempt)),
    }
}

fn jitter(base_seconds: i64, attempt: i32) -> i64 {
    let percent = 80 + (attempt as i64 * 37 % 41);
    base_seconds * percent / 100
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use nvbes_email::EmailTemplate;

    use super::{business_type, retry_policy};

    #[test]
    fn credential_retries_fit_short_lived_codes() {
        let first = retry_policy("credential", 1);
        let last = retry_policy("credential", 4);
        assert_eq!(first.maximum_attempts, 4);
        assert!(first.retry_delay.num_seconds() < 20);
        assert!(last.retry_delay.num_minutes() < 4);
    }

    #[test]
    fn test_capture_uses_the_existing_beta_business_type() {
        assert_eq!(
            business_type(&EmailTemplate::EmailVerificationV1 {
                user_name: "Ada".into(),
                verification_url: "https://account.nvbes.fr/verify-result?token=secret".into(),
                credential_expires_at: Utc::now(),
            }),
            "verification"
        );
    }
}
