use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{AccountError, AccountResult};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct EmailDeliveryRecord {
    pub business_type: String,
    pub recipient_hash: String,
    pub message_id: String,
    pub status: String,
    pub provider_email_id: Option<String>,
    pub last_error_class: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_summary: Option<String>,
}

pub async fn ensure_email_delivery(
    db: &sqlx::PgPool,
    job_id: Uuid,
    business_type: &str,
    recipient_email: &str,
    message_id: &str,
) -> AccountResult<()> {
    let recipient_hash = recipient_hash(recipient_email);
    let mut tx = db.begin().await.map_err(AccountError::from)?;

    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id,
            business_type,
            recipient_email,
            recipient_hash,
            message_id,
            status,
            sent_at
        )
        VALUES ($1, $2, $3, $4, $5, 'queued', NULL)
        ON CONFLICT (job_id) DO NOTHING
        "#,
    )
    .bind(job_id)
    .bind(business_type)
    .bind(recipient_email)
    .bind(&recipient_hash)
    .bind(message_id)
    .execute(&mut *tx)
    .await
    .map_err(AccountError::from)?;

    let record = fetch_email_delivery_tx(&mut tx, job_id, false).await?;
    validate_identity(&record, business_type, &recipient_hash, message_id)?;
    tx.commit().await.map_err(AccountError::from)
}

pub async fn completed_email_delivery(
    db: &sqlx::PgPool,
    job_id: Uuid,
) -> AccountResult<Option<String>> {
    let result = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT provider_email_id
        FROM email_messages
        WHERE job_id = $1
          AND provider_email_id IS NOT NULL
          AND status IN ('sent', 'delivered', 'bounced', 'complained', 'dropped')
        "#,
    )
    .bind(job_id)
    .fetch_optional(db)
    .await
    .map_err(AccountError::from)?
    .flatten();

    Ok(result)
}

pub async fn lock_email_delivery_tx(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    business_type: &str,
    recipient_email: &str,
    message_id: &str,
) -> AccountResult<EmailDeliveryRecord> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(job_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(AccountError::from)?;

    let record = fetch_email_delivery_tx(tx, job_id, true).await?;
    validate_identity(
        &record,
        business_type,
        &recipient_hash(recipient_email),
        message_id,
    )?;
    Ok(record)
}

pub async fn mark_email_delivery_attempt_tx(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
) -> AccountResult<()> {
    require_one_row(
        sqlx::query(
            r#"
            UPDATE email_messages
            SET status = 'sending',
                attempt_count = attempt_count + 1,
                last_attempt_at = NOW(),
                last_error_class = NULL,
                last_error_code = NULL,
                last_error_summary = NULL,
                updated_at = NOW()
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .execute(&mut **tx)
        .await
        .map_err(AccountError::from)?
        .rows_affected(),
    )
}

pub async fn record_email_delivery_failure_tx(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    error_class: &str,
    error_code: &str,
    error_summary: &str,
) -> AccountResult<()> {
    require_one_row(
        sqlx::query(
            r#"
            UPDATE email_messages
            SET status = 'failed',
                last_error_class = $2,
                last_error_code = $3,
                last_error_summary = $4,
                updated_at = NOW()
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind(error_class)
        .bind(error_code)
        .bind(error_summary)
        .execute(&mut **tx)
        .await
        .map_err(AccountError::from)?
        .rows_affected(),
    )
}

pub async fn record_email_delivery_success_tx(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    provider_email_id: &str,
) -> AccountResult<()> {
    require_one_row(
        sqlx::query(
            r#"
            UPDATE email_messages
            SET status = 'sent',
                provider_email_id = $2,
                sent_at = COALESCE(sent_at, NOW()),
                last_error_class = NULL,
                last_error_code = NULL,
                last_error_summary = NULL,
                updated_at = NOW()
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind(provider_email_id)
        .execute(&mut **tx)
        .await
        .map_err(AccountError::from)?
        .rows_affected(),
    )
}

async fn fetch_email_delivery_tx(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    for_update: bool,
) -> AccountResult<EmailDeliveryRecord> {
    let suffix = if for_update { " FOR UPDATE" } else { "" };
    sqlx::query_as::<_, EmailDeliveryRecord>(&format!(
        r#"
        SELECT business_type,
               recipient_hash,
               message_id,
               status::text AS status,
               provider_email_id,
               last_error_class,
               last_error_code,
               last_error_summary
        FROM email_messages
        WHERE job_id = $1{suffix}
        "#
    ))
    .bind(job_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(AccountError::from)
}

fn validate_identity(
    record: &EmailDeliveryRecord,
    business_type: &str,
    recipient_hash: &str,
    message_id: &str,
) -> AccountResult<()> {
    if record.business_type == business_type
        && record.recipient_hash == recipient_hash
        && record.message_id == message_id
    {
        return Ok(());
    }

    Err(AccountError::conflict(
        "email_job_identity_conflict",
        "Email job identifier is already bound to another delivery",
    ))
}

fn recipient_hash(recipient_email: &str) -> String {
    hex::encode(Sha256::digest(recipient_email.as_bytes()))
}

fn require_one_row(rows_affected: u64) -> AccountResult<()> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(AccountError::not_found(
            "email_delivery_not_found",
            "Email delivery ledger entry was not found",
        ))
    }
}
