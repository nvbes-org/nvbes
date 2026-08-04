use chrono::{DateTime, Utc};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use nvbes_email::{EmailCommand, EmailReceipt, proto::nvbes::email::v1::SubmitEmailRequest};

use crate::crypto::EmailCrypto;

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .min_connections(1)
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await
        .map_err(anyhow::Error::from)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn database_now(pool: &PgPool) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await
}

pub async fn accept_command(
    pool: &PgPool,
    crypto: &EmailCrypto,
    message_id_domain: &str,
    wire: &SubmitEmailRequest,
    command: &EmailCommand,
) -> Result<EmailReceipt, AcceptCommandError> {
    let now = database_now(pool).await?;
    command.validate(now)?;
    let message_uuid = Uuid::new_v4();
    let message_id = format!("<{message_uuid}@{message_id_domain}>");
    let fingerprint = command_fingerprint(wire);
    let recipient = wire.recipient.as_ref().ok_or(AcceptCommandError::Invalid)?;
    let template = wire.template.as_ref().ok_or(AcceptCommandError::Invalid)?;
    let recipient_sealed = crypto.seal(message_uuid, "recipient", &recipient.encode_to_vec())?;
    let template_sealed = crypto.seal(message_uuid, "template", &template.encode_to_vec())?;
    let recipient_hash = crypto.recipient_hash(&recipient.email);
    let (template_name, template_version) = command.template.name_and_version();

    let inserted = sqlx::query_as::<_, (DateTime<Utc>,)>(
        r#"
        INSERT INTO email_messages (
            id, producer, idempotency_key, command_fingerprint, request_id, correlation_id,
            category, template_name, template_version, recipient_hash,
            recipient_ciphertext, recipient_nonce, template_ciphertext, template_nonce,
            deliver_before, message_id, next_attempt_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7::email_category, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, clock_timestamp()
        )
        ON CONFLICT (producer, idempotency_key) DO NOTHING
        RETURNING accepted_at
        "#,
    )
    .bind(message_uuid)
    .bind(&command.producer)
    .bind(command.idempotency_key.as_str())
    .bind(fingerprint.as_slice())
    .bind(&command.context.request_id)
    .bind(&command.context.correlation_id)
    .bind(command.category.as_str())
    .bind(template_name)
    .bind(template_version)
    .bind(recipient_hash.as_slice())
    .bind(recipient_sealed.ciphertext)
    .bind(recipient_sealed.nonce.as_slice())
    .bind(template_sealed.ciphertext)
    .bind(template_sealed.nonce.as_slice())
    .bind(command.deliver_before)
    .bind(&message_id)
    .fetch_optional(pool)
    .await?;

    if let Some((accepted_at,)) = inserted {
        return Ok(EmailReceipt {
            message_id,
            accepted_at,
            deliver_before: command.deliver_before,
            duplicate: false,
        });
    }

    existing_receipt(pool, command, &fingerprint).await
}

async fn existing_receipt(
    pool: &PgPool,
    command: &EmailCommand,
    fingerprint: &[u8; 32],
) -> Result<EmailReceipt, AcceptCommandError> {
    let row = sqlx::query_as::<_, (Uuid, String, Vec<u8>, DateTime<Utc>, DateTime<Utc>)>(
        r#"
        SELECT id, message_id, command_fingerprint, accepted_at, deliver_before
        FROM email_messages
        WHERE producer = $1 AND idempotency_key = $2
        "#,
    )
    .bind(&command.producer)
    .bind(command.idempotency_key.as_str())
    .fetch_one(pool)
    .await?;

    if !constant_time_eq(&row.2, fingerprint) {
        return Err(AcceptCommandError::Conflict);
    }
    Ok(EmailReceipt {
        message_id: row.1,
        accepted_at: row.3,
        deliver_before: row.4,
        duplicate: true,
    })
}

pub async fn expire_stale_messages(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE email_messages
        SET state = 'expired', terminal_at = clock_timestamp(), updated_at = clock_timestamp()
        WHERE (
            state IN ('accepted', 'deferred')
            OR (state = 'dispatching' AND lease_expires_at <= clock_timestamp())
        )
          AND deliver_before <= clock_timestamp()
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn queue_snapshot(pool: &PgPool) -> Result<(i64, Option<f64>), sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT,
               EXTRACT(EPOCH FROM clock_timestamp() - MIN(accepted_at))::DOUBLE PRECISION
        FROM email_messages
        WHERE state IN ('accepted', 'deferred', 'dispatching')
        "#,
    )
    .fetch_one(pool)
    .await
}

#[derive(Debug, Default)]
pub struct RetentionResult {
    pub payloads: u64,
    pub diagnostics: u64,
    pub messages: u64,
    pub events: u64,
}

pub async fn apply_retention(
    pool: &PgPool,
    payload_days: i32,
    ledger_days: i32,
) -> Result<RetentionResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let payloads = sqlx::query(
        r#"
        UPDATE email_messages
        SET recipient_ciphertext = NULL, recipient_nonce = NULL,
            template_ciphertext = NULL, template_nonce = NULL,
            payload_purged_at = clock_timestamp(), updated_at = clock_timestamp()
        WHERE terminal_at <= clock_timestamp() - ($1 * INTERVAL '1 day')
          AND payload_purged_at IS NULL
        "#,
    )
    .bind(payload_days)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let diagnostics = sqlx::query(
        r#"
        UPDATE email_provider_events
        SET diagnostic = '{}'::jsonb, diagnostic_purged_at = clock_timestamp()
        WHERE processed_at <= clock_timestamp() - ($1 * INTERVAL '1 day')
          AND diagnostic_purged_at IS NULL
        "#,
    )
    .bind(payload_days)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let messages = sqlx::query(
        r#"
        DELETE FROM email_messages
        WHERE terminal_at <= clock_timestamp() - ($1 * INTERVAL '1 day')
          AND NOT EXISTS (
              SELECT 1 FROM email_suppressions AS suppression
              WHERE suppression.recipient_hash = email_messages.recipient_hash
                AND suppression.active
          )
        "#,
    )
    .bind(ledger_days)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let events = sqlx::query(
        r#"
        DELETE FROM email_provider_events
        WHERE processed_at <= clock_timestamp() - ($1 * INTERVAL '1 day')
        "#,
    )
    .bind(ledger_days)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(RetentionResult {
        payloads,
        diagnostics,
        messages,
        events,
    })
}

pub async fn release_suppression_by_message(
    pool: &PgPool,
    message_id: Uuid,
    actor: &str,
    reason: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE email_suppressions AS suppression
        SET active = FALSE, released_at = clock_timestamp(), released_by = $2,
            release_reason = $3
        FROM email_messages AS message
        WHERE message.id = $1
          AND suppression.recipient_hash = message.recipient_hash
          AND suppression.active
        "#,
    )
    .bind(message_id)
    .bind(actor)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

fn command_fingerprint(wire: &SubmitEmailRequest) -> [u8; 32] {
    let mut immutable = wire.clone();
    immutable.context = None;
    immutable.idempotency_key.clear();
    Sha256::digest(immutable.encode_to_vec()).into()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptCommandError {
    #[error("invalid email command")]
    Invalid,
    #[error("idempotency key conflicts with another email command")]
    Conflict,
    #[error("email command persistence failed")]
    Database(#[from] sqlx::Error),
    #[error("email command encryption failed")]
    Encryption(#[from] anyhow::Error),
    #[error("email command validation failed")]
    Validation(#[from] nvbes_email::EmailCommandError),
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.database.tests.rs"]
mod tests;
