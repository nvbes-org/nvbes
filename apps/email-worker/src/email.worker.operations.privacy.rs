use chrono::{DateTime, Utc};
use nvbes_email::proto::nvbes::email::v1::{
    EmailPrivacyActivity, EmailPrivacyMessage, EmailPrivacyProviderEvent,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::EmailCrypto;

type PrivacyMessageRow = (
    Uuid,
    String,
    String,
    i16,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
    DateTime<Utc>,
);

type PrivacyEventRow = (
    Uuid,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<String>,
);

pub async fn load(
    db: &PgPool,
    crypto: &EmailCrypto,
    email: &str,
) -> Result<EmailPrivacyActivity, sqlx::Error> {
    let recipient_hash = crypto.recipient_hash(email);
    let messages = sqlx::query_as::<_, PrivacyMessageRow>(
        r#"
        SELECT id, producer, template_name, template_version, state::text, accepted_at,
               provider_accepted_at, terminal_at, deliver_before, updated_at
        FROM email_messages
        WHERE recipient_hash = $1
        ORDER BY accepted_at DESC
        "#,
    )
    .bind(recipient_hash.as_slice())
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| EmailPrivacyMessage {
        id: row.0.to_string(),
        producer: row.1,
        business_type: row.2,
        template_version: i32::from(row.3),
        status: row.4,
        accepted_at: Some(timestamp(row.5)),
        provider_accepted_at: row.6.map(timestamp),
        terminal_at: row.7.map(timestamp),
        deliver_before: Some(timestamp(row.8)),
        updated_at: Some(timestamp(row.9)),
    })
    .collect();

    let provider_events = sqlx::query_as::<_, PrivacyEventRow>(
        r#"
        SELECT DISTINCT event.id, event.provider, event.event_type, event.received_at,
               event.processed_at, event.processing_result
        FROM email_provider_events AS event
        JOIN email_messages AS message
          ON message.provider_message_id = event.provider_message_id
        WHERE message.recipient_hash = $1
        ORDER BY event.received_at DESC
        "#,
    )
    .bind(recipient_hash.as_slice())
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| EmailPrivacyProviderEvent {
        id: row.0.to_string(),
        provider: row.1,
        event_type: row.2,
        received_at: Some(timestamp(row.3)),
        processed_at: row.4.map(timestamp),
        processing_result: row.5,
    })
    .collect();

    Ok(EmailPrivacyActivity {
        messages,
        provider_events,
    })
}

fn timestamp(value: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}
