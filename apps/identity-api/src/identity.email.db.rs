use serde_json::Value;

use super::webhooks::ScalewayEmailEvent;
use crate::http::error::AppError;

pub async fn record_email_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &ScalewayEmailEvent,
) -> Result<(), AppError> {
    let details = build_event_details(event);

    sqlx::query(
        r#"
        INSERT INTO email_events (
          provider_event_id,
          provider_email_id,
          email,
          event_type,
          occurred_at,
          details
        )
        VALUES ($1, $2, $3, $4::email_event_type, $5::timestamptz, $6)
        ON CONFLICT (provider_event_id) DO NOTHING
        "#,
    )
    .bind(&event.id)
    .bind(&event.email_id)
    .bind(&event.email)
    .bind(&event.event_type)
    .bind(&event.timestamp)
    .bind(sqlx::types::Json(details))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn record_email_sent_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    provider_email_id: &str,
    subject: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO email_events (
          provider_event_id,
          provider_email_id,
          email,
          event_type,
          occurred_at,
          details
        )
        VALUES ($1, $2, $3, 'email_sent'::email_event_type, NOW(), $4)
        ON CONFLICT (provider_event_id) DO NOTHING
        "#,
    )
    .bind(format!("sent:{provider_email_id}"))
    .bind(provider_email_id)
    .bind(email)
    .bind(sqlx::types::Json(serde_json::json!({"subject": subject})))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn mark_email_event_processed_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider_event_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE email_events
        SET processed_at = NOW()
        WHERE provider_event_id = $1
        "#,
    )
    .bind(provider_event_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn suppress_email_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    reason: &str,
    details: Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO suppressed_emails (email, reason, details)
        VALUES ($1, $2, $3)
        ON CONFLICT (email) DO UPDATE
        SET reason = CASE
              WHEN suppressed_emails.reason = 'hard_bounce' THEN suppressed_emails.reason
              ELSE $2
            END,
            details = CASE
              WHEN suppressed_emails.reason = 'hard_bounce' THEN suppressed_emails.details
              ELSE $3
            END,
            suppressed_at = CASE
              WHEN suppressed_emails.reason = 'hard_bounce' THEN suppressed_emails.suppressed_at
              ELSE NOW()
            END
        "#,
    )
    .bind(email)
    .bind(reason)
    .bind(sqlx::types::Json(details))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn is_email_suppressed(db: &sqlx::PgPool, email: &str) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM suppressed_emails WHERE email = $1
        )
        "#,
    )
    .bind(email)
    .fetch_one(db)
    .await?;

    Ok(exists)
}

pub async fn record_email_message_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: uuid::Uuid,
    business_type: &str,
    recipient_email: &str,
    provider_email_id: &str,
) -> Result<(), AppError> {
    use sha2::{Digest, Sha256};

    let recipient_hash = hex::encode(Sha256::digest(recipient_email.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id,
            business_type,
            recipient_email,
            recipient_hash,
            provider_email_id,
            status
        )
        VALUES ($1, $2, $3, $4, $5, 'sent')
        "#,
    )
    .bind(job_id)
    .bind(business_type)
    .bind(recipient_email)
    .bind(recipient_hash)
    .bind(provider_email_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn update_email_message_status_by_provider_id_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider_email_id: &str,
    status: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE email_messages
        SET status = $2::email_message_status,
            updated_at = NOW()
        WHERE provider_email_id = $1
        "#,
    )
    .bind(provider_email_id)
    .bind(status)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn update_email_message_status_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: uuid::Uuid,
    status: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE email_messages
        SET status = $2::email_message_status,
            updated_at = NOW()
        WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .bind(status)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn build_event_details(event: &ScalewayEmailEvent) -> Value {
    let mut details = serde_json::Map::new();
    if let Some(ref msg) = event.message {
        if let Some(ref subject) = msg.subject {
            details.insert("subject".to_string(), Value::String(subject.clone()));
        }
        if let Some(ref message_id) = msg.message_id {
            details.insert("message_id".to_string(), Value::String(message_id.clone()));
        }
    }
    Value::Object(details)
}
