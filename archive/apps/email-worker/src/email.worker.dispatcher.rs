use std::time::{Duration, Instant};

use chrono::Duration as ChronoDuration;
use nvbes_email::{
    EmailAddress, EmailFailureClass, EmailMessage, EmailTemplate,
    proto::nvbes::email::v1::{EmailRecipient, TransactionalEmailTemplate},
};
use prost::Message;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::{
    crypto::SealedValue,
    dispatch_attempt::FailureResolution,
    dispatch_db::{self, ClaimResult},
    email_metrics, error_reporting,
    state::EmailWorkerState,
};

pub async fn run_local(
    state: EmailWorkerState,
    mut receiver: mpsc::Receiver<Uuid>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            message_id = receiver.recv() => {
                let Some(message_id) = message_id else {
                    break;
                };
                match dispatch_message(&state, message_id).await {
                    Ok(DispatchOutcome::Acknowledged) => {}
                    Ok(DispatchOutcome::Retry) => {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        if let Err(error) = state.dispatch_queue.enqueue(message_id).await {
                            tracing::error!(%message_id, error = ?error, "local email retry enqueue failed");
                        }
                    }
                    Err(error) => tracing::error!(%message_id, error = ?error, "local email dispatch failed"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    Acknowledged,
    Retry,
}

pub async fn dispatch_message(
    state: &EmailWorkerState,
    message_id: Uuid,
) -> anyhow::Result<DispatchOutcome> {
    let claim = match dispatch_db::claim_message(&state.db, message_id).await? {
        ClaimResult::Claimed(claim) => claim,
        ClaimResult::Busy => return Ok(DispatchOutcome::Retry),
        ClaimResult::Settled | ClaimResult::Missing => return Ok(DispatchOutcome::Acknowledged),
    };
    if dispatch_db::expire_claim_if_due(&state.db, &claim).await? {
        return Ok(DispatchOutcome::Acknowledged);
    }

    dispatch_claim(state, &claim).await
}

#[tracing::instrument(
    name = "email.delivery",
    skip_all,
    fields(
        email.provider = state.config.provider.label(),
        email.business_type = claim.template_name,
        email.attempt = claim.attempt_count,
        otel.status_code = tracing::field::Empty,
    )
)]
async fn dispatch_claim(
    state: &EmailWorkerState,
    claim: &dispatch_db::ClaimedEmail,
) -> anyhow::Result<DispatchOutcome> {
    let message = match materialize(state, claim) {
        Ok(message) => message,
        Err(error) => {
            tracing::Span::current().record("otel.status_code", "ERROR");
            let policy = retry_policy(&claim.category, claim.attempt_count);
            error_reporting::capture_delivery(
                &state.config,
                claim.id,
                &claim.template_name,
                claim.attempt_count.max(0) as u32,
                policy.maximum_attempts.max(0) as u32,
                error.as_ref(),
            );
            tracing::error!(message_id = %claim.id, error = ?error, "stored email command is invalid");
            let _ = dispatch_db::complete_failure(
                &state.db,
                claim,
                "permanent_failure",
                "stored_command_invalid",
                1,
                ChronoDuration::zero(),
            )
            .await?;
            email_metrics::dispatch(
                state.config.provider.label(),
                &claim.template_name,
                "permanent_failure",
                Duration::ZERO,
            );
            return Ok(DispatchOutcome::Acknowledged);
        }
    };

    if dispatch_db::expire_claim_if_due(&state.db, claim).await? {
        return Ok(DispatchOutcome::Acknowledged);
    }
    let started_at = Instant::now();
    match state
        .provider
        .send_message_before(&message, claim.deliver_before)
        .await
    {
        Ok(result) => {
            dispatch_db::complete_success(&state.db, claim, &result.provider_email_id).await?;
            email_metrics::dispatch(
                state.config.provider.label(),
                &claim.template_name,
                "provider_accepted",
                started_at.elapsed(),
            );
        }
        Err(error) => {
            tracing::Span::current().record("otel.status_code", "ERROR");
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
            let resolution = dispatch_db::complete_failure(
                &state.db,
                claim,
                outcome,
                error.safe_code(),
                policy.maximum_attempts,
                policy.retry_delay,
            )
            .await?;
            email_metrics::dispatch(
                state.config.provider.label(),
                &claim.template_name,
                outcome,
                started_at.elapsed(),
            );
            if matches!(resolution, FailureResolution::RetryAt(_)) {
                return Ok(DispatchOutcome::Retry);
            }
        }
    }
    Ok(DispatchOutcome::Acknowledged)
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
        EmailTemplate::OperationalReadinessV1 { .. } => "operational_readiness",
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
    let _ = (category, attempt);
    RetryPolicy {
        maximum_attempts: 4,
        retry_delay: ChronoDuration::seconds(60),
    }
}

#[cfg(test)]
#[path = "email.worker.dispatcher.unit.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.dispatcher.tests.rs"]
mod database_tests;
