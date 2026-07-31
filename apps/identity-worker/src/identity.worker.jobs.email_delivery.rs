use nvbes_email::{EmailMessage, EmailSender};
use nvbes_product_identity::email::db::{
    ensure_email_delivery, lock_email_delivery_tx, mark_email_delivery_attempt_tx,
    record_email_delivery_failure_tx, record_email_delivery_success_tx, record_email_sent_event_tx,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::worker::job_failure::JobExecutionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EmailDeliveryOutcome {
    pub provider_email_id: String,
    pub deduplicated: bool,
}

/// Delivers with durable local deduplication and a stable RFC 5322 Message-ID.
///
/// The transaction lock prevents concurrent local sends for one job. SMTP
/// acceptance followed by a failed database commit remains an unavoidable
/// at-least-once ambiguity; a retry transmits the same identifiers.
pub(super) async fn deliver_email(
    db: &PgPool,
    sender: &dyn EmailSender,
    job_id: Uuid,
    business_type: &str,
    recipient_email: &str,
    subject: &str,
    mut message: EmailMessage,
) -> Result<EmailDeliveryOutcome, JobExecutionError> {
    let message_id = deterministic_message_id(job_id);
    set_header(&mut message, "Message-ID", &message_id);
    set_header(&mut message, "X-Nvbes-Email-Job-Id", &job_id.to_string());
    set_header(&mut message, "X-Nvbes-Email-Business-Type", business_type);

    ensure_email_delivery(db, job_id, business_type, recipient_email, &message_id)
        .await
        .map_err(|error| JobExecutionError::from_identity(&error))?;

    let mut tx = db.begin().await.map_err(|_| {
        JobExecutionError::transient(
            "email_ledger_transaction",
            "Email delivery transaction could not start",
        )
    })?;
    let record =
        lock_email_delivery_tx(&mut tx, job_id, business_type, recipient_email, &message_id)
            .await
            .map_err(|error| JobExecutionError::from_identity(&error))?;

    if let Some(provider_email_id) = completed_provider_id(&record) {
        tx.commit().await.map_err(|_| {
            JobExecutionError::transient(
                "email_ledger_commit",
                "Email delivery transaction could not commit",
            )
        })?;
        return Ok(EmailDeliveryOutcome {
            provider_email_id,
            deduplicated: true,
        });
    }
    if let Some(failure) = permanent_recorded_failure(&record) {
        tx.commit().await.map_err(|_| {
            JobExecutionError::transient(
                "email_ledger_commit",
                "Email delivery transaction could not commit",
            )
        })?;
        return Err(failure);
    }

    mark_email_delivery_attempt_tx(&mut tx, job_id)
        .await
        .map_err(|error| JobExecutionError::from_identity(&error))?;

    let result = match sender.send_message(&message).await {
        Ok(result) => result,
        Err(error) => {
            let failure = JobExecutionError::from_email(&error);
            record_email_delivery_failure_tx(
                &mut tx,
                job_id,
                failure.class().as_str(),
                failure.code(),
                failure.summary(),
            )
            .await
            .map_err(|error| JobExecutionError::from_identity(&error))?;
            tx.commit().await.map_err(|_| {
                JobExecutionError::transient(
                    "email_ledger_commit",
                    "Email failure state could not commit",
                )
            })?;
            return Err(failure);
        }
    };
    let provider_email_id = if result.provider_email_id.trim().is_empty() {
        message_id
    } else {
        result.provider_email_id
    };

    record_email_delivery_success_tx(&mut tx, job_id, &provider_email_id)
        .await
        .map_err(|error| JobExecutionError::from_identity(&error))?;
    record_email_sent_event_tx(&mut tx, recipient_email, &provider_email_id, subject)
        .await
        .map_err(|error| JobExecutionError::from_identity(&error))?;
    tx.commit().await.map_err(|_| {
        JobExecutionError::transient(
            "email_ledger_commit",
            "Email success state could not commit",
        )
    })?;

    Ok(EmailDeliveryOutcome {
        provider_email_id,
        deduplicated: false,
    })
}

pub(super) fn deterministic_message_id(job_id: Uuid) -> String {
    format!("<identity-job-{job_id}@worker.nvbes.fr>")
}

fn completed_provider_id(
    record: &nvbes_product_identity::email::db::EmailDeliveryRecord,
) -> Option<String> {
    let completed = matches!(
        record.status.as_str(),
        "sent" | "delivered" | "bounced" | "complained" | "dropped"
    );
    completed
        .then(|| record.provider_email_id.clone())
        .flatten()
}

fn permanent_recorded_failure(
    record: &nvbes_product_identity::email::db::EmailDeliveryRecord,
) -> Option<JobExecutionError> {
    if record.status != "failed" || record.last_error_class.as_deref() != Some("permanent") {
        return None;
    }
    Some(JobExecutionError::from_stored(
        "permanent",
        record
            .last_error_code
            .as_deref()
            .unwrap_or("email_delivery_permanent"),
        record
            .last_error_summary
            .as_deref()
            .unwrap_or("Email delivery permanently failed"),
    ))
}

fn set_header(message: &mut EmailMessage, name: &str, value: &str) {
    message
        .headers
        .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
    message.headers.push((name.to_string(), value.to_string()));
}

#[cfg(test)]
#[path = "identity.worker.jobs.email_delivery.migration.tests.rs"]
mod migration_tests;
#[cfg(test)]
#[path = "identity.worker.jobs.email_delivery.tests.rs"]
mod tests;
