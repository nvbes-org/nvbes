use chrono::{DateTime, Utc};
use nvbes_email::proto::nvbes::email::v1::{
    EmailBusinessTypeCount, EmailFailureSummary, EmailOperationsSnapshot,
    EmailProviderEventSummary, EmailRecipient, EmailStatusCount, EmailSuppressionSummary,
};
use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::{EmailCrypto, SealedValue};

pub async fn load(
    db: &PgPool,
    crypto: &EmailCrypto,
) -> Result<EmailOperationsSnapshot, anyhow::Error> {
    let metrics = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE state IN ('accepted', 'deferred', 'dispatching')),
          COUNT(*) FILTER (WHERE provider_accepted_at >= clock_timestamp() - INTERVAL '24 hours'),
          COUNT(*) FILTER (WHERE state = 'delivered' AND terminal_at >= clock_timestamp() - INTERVAL '24 hours'),
          COUNT(*) FILTER (WHERE state IN ('failed', 'hard_bounced', 'complained', 'dropped')
                            AND terminal_at >= clock_timestamp() - INTERVAL '24 hours'),
          (SELECT COUNT(*) FROM email_suppressions WHERE active),
          (SELECT COUNT(*) FROM email_provider_events
           WHERE received_at >= clock_timestamp() - INTERVAL '24 hours'),
          (SELECT COUNT(*) FROM email_provider_events WHERE processed_at IS NULL),
          (SELECT COUNT(*) FROM email_provider_events
           WHERE event_type IN (
             'email_deferred', 'email_soft_bounced', 'email_mailbox_not_found',
             'email_blocklisted', 'blocklist_created'
           ) AND received_at >= clock_timestamp() - INTERVAL '24 hours')
        FROM email_messages
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(EmailOperationsSnapshot {
        queued_message_count: metrics.0,
        sent_message_count_24h: metrics.1,
        delivered_message_count_24h: metrics.2,
        failed_message_count_24h: metrics.3,
        suppressed_email_count: metrics.4,
        webhook_event_count_24h: metrics.5,
        unprocessed_event_count: metrics.6,
        status_distribution: status_distribution(db).await?,
        business_type_distribution: business_type_distribution(db).await?,
        recent_failures: recent_failures(db, crypto).await?,
        recent_suppressions: recent_suppressions(db, crypto).await?,
        recent_unprocessed_events: recent_unprocessed_events(db, crypto).await?,
        bounce_event_count_24h: metrics.7,
    })
}

async fn status_distribution(db: &PgPool) -> Result<Vec<EmailStatusCount>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT state::text, COUNT(*) FROM email_messages GROUP BY state ORDER BY COUNT(*) DESC, state",
    )
    .fetch_all(db)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(status, message_count)| EmailStatusCount {
                status,
                message_count,
            })
            .collect()
    })
}

async fn business_type_distribution(
    db: &PgPool,
) -> Result<Vec<EmailBusinessTypeCount>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64)>(
        r#"
        SELECT template_name, COUNT(*),
          COUNT(*) FILTER (WHERE state IN ('failed', 'hard_bounced', 'complained', 'dropped'))
        FROM email_messages
        GROUP BY template_name
        ORDER BY COUNT(*) FILTER (
          WHERE state IN ('failed', 'hard_bounced', 'complained', 'dropped')
        ) DESC, COUNT(*) DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(
                |(business_type, message_count, failure_count)| EmailBusinessTypeCount {
                    business_type,
                    message_count,
                    failure_count,
                },
            )
            .collect()
    })
}

type EncryptedFailure = (
    Uuid,
    String,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    String,
    DateTime<Utc>,
);

async fn recent_failures(
    db: &PgPool,
    crypto: &EmailCrypto,
) -> Result<Vec<EmailFailureSummary>, anyhow::Error> {
    let rows = sqlx::query_as::<_, EncryptedFailure>(
        r#"
        SELECT id, template_name, recipient_ciphertext, recipient_nonce,
               provider_message_id, state::text, updated_at
        FROM email_messages
        WHERE state IN ('failed', 'hard_bounced', 'complained', 'dropped')
        ORDER BY updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(EmailFailureSummary {
                id: row.0.to_string(),
                business_type: row.1,
                recipient_email: decrypt_recipient(crypto, row.0, row.2, row.3)?,
                provider_email_id: row.4,
                status: row.5,
                updated_at: Some(timestamp(row.6)),
            })
        })
        .collect()
}

type EncryptedSuppression = (
    Uuid,
    Vec<u8>,
    Vec<u8>,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<String>,
);

async fn recent_suppressions(
    db: &PgPool,
    crypto: &EmailCrypto,
) -> Result<Vec<EmailSuppressionSummary>, anyhow::Error> {
    let rows = sqlx::query_as::<_, EncryptedSuppression>(
        r#"
        SELECT recipient_encryption_id, recipient_ciphertext, recipient_nonce, reason,
               suppressed_at, reviewed_at, reviewed_by
        FROM email_suppressions
        WHERE active
        ORDER BY suppressed_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(EmailSuppressionSummary {
                email: decrypt_recipient(crypto, row.0, Some(row.1), Some(row.2))?,
                reason: row.3,
                suppressed_at: Some(timestamp(row.4)),
                reviewed_at: row.5.map(timestamp),
                reviewed_by: row.6,
            })
        })
        .collect()
}

type UnprocessedEvent = (
    Uuid,
    String,
    Option<String>,
    String,
    DateTime<Utc>,
    Option<Uuid>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
);

async fn recent_unprocessed_events(
    db: &PgPool,
    crypto: &EmailCrypto,
) -> Result<Vec<EmailProviderEventSummary>, anyhow::Error> {
    let rows = sqlx::query_as::<_, UnprocessedEvent>(
        r#"
        SELECT event.id, event.provider_event_id, event.provider_message_id, event.event_type,
               event.received_at, message.id, message.recipient_ciphertext, message.recipient_nonce
        FROM email_provider_events AS event
        LEFT JOIN email_messages AS message
          ON message.provider_message_id = event.provider_message_id
        WHERE event.processed_at IS NULL
        ORDER BY event.received_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            let email = match row.5 {
                Some(message_id) => decrypt_recipient(crypto, message_id, row.6, row.7)?,
                None => String::new(),
            };
            Ok(EmailProviderEventSummary {
                id: row.0.to_string(),
                provider_event_id: row.1,
                provider_email_id: row.2,
                email,
                event_type: row.3,
                received_at: Some(timestamp(row.4)),
            })
        })
        .collect()
}

fn decrypt_recipient(
    crypto: &EmailCrypto,
    encryption_id: Uuid,
    ciphertext: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
) -> Result<String, anyhow::Error> {
    let (Some(ciphertext), Some(nonce)) = (ciphertext, nonce) else {
        return Ok("redacted".to_string());
    };
    let nonce = nonce
        .try_into()
        .map_err(|_| anyhow::anyhow!("email recipient nonce is invalid"))?;
    let plaintext = crypto.open(
        encryption_id,
        "recipient",
        &SealedValue { ciphertext, nonce },
    )?;
    Ok(EmailRecipient::decode(plaintext.as_slice())?.email)
}

fn timestamp(value: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.operations.snapshot.tests.rs"]
mod tests;
