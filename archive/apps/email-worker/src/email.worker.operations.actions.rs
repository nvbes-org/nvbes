use nvbes_email::proto::nvbes::email::v1::EmailRecipient;
use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::EmailCrypto;

pub struct OperatorAction<'a> {
    pub actor: &'a str,
    pub reason: &'a str,
}

pub struct ActionReceipt {
    pub id: Uuid,
    pub status: &'static str,
}

pub async fn replay_email(
    db: &PgPool,
    message_id: Uuid,
    operator: OperatorAction<'_>,
) -> Result<ActionReceipt, ActionError> {
    let mut tx = db.begin().await?;
    let replayed = sqlx::query(
        r#"
        UPDATE email_messages AS message
        SET state = 'accepted', next_attempt_at = clock_timestamp(), terminal_at = NULL,
            lease_token = NULL, lease_expires_at = NULL, updated_at = clock_timestamp()
        WHERE id = $1
          AND state IN ('failed', 'dropped', 'hard_bounced')
          AND deliver_before > clock_timestamp()
          AND payload_purged_at IS NULL
          AND NOT EXISTS (
              SELECT 1 FROM email_suppressions AS suppression
              WHERE suppression.recipient_hash = message.recipient_hash
                AND suppression.active
          )
        "#,
    )
    .bind(message_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if replayed == 0 {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM email_messages WHERE id = $1)")
                .bind(message_id)
                .fetch_one(&mut *tx)
                .await?;
        return Err(if exists {
            ActionError::Precondition
        } else {
            ActionError::NotFound
        });
    }
    let receipt = record(
        &mut tx,
        "replay_email",
        operator,
        Some(message_id),
        None,
        "queued",
    )
    .await?;
    tx.commit().await?;
    Ok(receipt)
}

pub async fn apply_suppression(
    db: &PgPool,
    crypto: &EmailCrypto,
    email: &str,
    scope: &str,
    operator: OperatorAction<'_>,
) -> Result<ActionReceipt, ActionError> {
    validate_email(email)?;
    if !matches!(scope, "all" | "optional") {
        return Err(ActionError::Invalid);
    }
    let encryption_id = Uuid::new_v4();
    let recipient = EmailRecipient {
        email: email.trim().to_ascii_lowercase(),
        name: None,
    };
    let sealed = crypto.seal(encryption_id, "recipient", &recipient.encode_to_vec())?;
    let recipient_hash = crypto.recipient_hash(email);
    let mut tx = db.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO email_suppressions (
            recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
            scope, reason, active, soft_bounce_count
        ) VALUES ($1, $2, $3, $4, $5, 'manual_operator', TRUE, 0)
        ON CONFLICT (recipient_hash) DO UPDATE
        SET recipient_ciphertext = EXCLUDED.recipient_ciphertext,
            recipient_nonce = EXCLUDED.recipient_nonce,
            recipient_encryption_id = EXCLUDED.recipient_encryption_id,
            scope = EXCLUDED.scope, reason = EXCLUDED.reason, active = TRUE,
            source_event_id = NULL, soft_bounce_count = 0,
            suppressed_at = clock_timestamp(), released_at = NULL,
            released_by = NULL, release_reason = NULL,
            reviewed_at = NULL, reviewed_by = NULL, review_reason = NULL
        "#,
    )
    .bind(recipient_hash.as_slice())
    .bind(sealed.ciphertext)
    .bind(sealed.nonce.as_slice())
    .bind(encryption_id)
    .bind(scope)
    .execute(&mut *tx)
    .await?;
    let receipt = record(
        &mut tx,
        "apply_suppression",
        operator,
        None,
        Some(recipient_hash.as_slice()),
        "applied",
    )
    .await?;
    tx.commit().await?;
    Ok(receipt)
}

pub async fn release_suppression(
    db: &PgPool,
    crypto: &EmailCrypto,
    email: &str,
    operator: OperatorAction<'_>,
) -> Result<ActionReceipt, ActionError> {
    validate_email(email)?;
    let recipient_hash = crypto.recipient_hash(email);
    let mut tx = db.begin().await?;
    let released = sqlx::query(
        r#"
        UPDATE email_suppressions
        SET active = FALSE, released_at = clock_timestamp(), released_by = $2,
            release_reason = $3
        WHERE recipient_hash = $1 AND active
        "#,
    )
    .bind(recipient_hash.as_slice())
    .bind(operator.actor)
    .bind(operator.reason)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let outcome = if released == 0 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM email_suppressions WHERE recipient_hash = $1)",
        )
        .bind(recipient_hash.as_slice())
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(ActionError::NotFound);
        }
        "already_released"
    } else {
        "released"
    };
    let receipt = record(
        &mut tx,
        "release_suppression",
        operator,
        None,
        Some(recipient_hash.as_slice()),
        outcome,
    )
    .await?;
    tx.commit().await?;
    Ok(receipt)
}

pub async fn review_suppression(
    db: &PgPool,
    crypto: &EmailCrypto,
    email: &str,
    operator: OperatorAction<'_>,
) -> Result<ActionReceipt, ActionError> {
    validate_email(email)?;
    let recipient_hash = crypto.recipient_hash(email);
    let mut tx = db.begin().await?;
    let reviewed = sqlx::query(
        r#"
        UPDATE email_suppressions
        SET reviewed_at = clock_timestamp(), reviewed_by = $2, review_reason = $3
        WHERE recipient_hash = $1
        "#,
    )
    .bind(recipient_hash.as_slice())
    .bind(operator.actor)
    .bind(operator.reason)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if reviewed == 0 {
        return Err(ActionError::NotFound);
    }
    let receipt = record(
        &mut tx,
        "review_suppression",
        operator,
        None,
        Some(recipient_hash.as_slice()),
        "reviewed",
    )
    .await?;
    tx.commit().await?;
    Ok(receipt)
}

async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    action_kind: &str,
    operator: OperatorAction<'_>,
    message_id: Option<Uuid>,
    recipient_hash: Option<&[u8]>,
    outcome: &'static str,
) -> Result<ActionReceipt, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO email_operator_actions (
            id, action_kind, actor, reason, message_id, recipient_hash, outcome
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(action_kind)
    .bind(operator.actor)
    .bind(operator.reason)
    .bind(message_id)
    .bind(recipient_hash)
    .bind(outcome)
    .execute(&mut **tx)
    .await?;
    Ok(ActionReceipt {
        id,
        status: outcome,
    })
}

fn validate_email(email: &str) -> Result<(), ActionError> {
    email
        .trim()
        .parse::<lettre::Address>()
        .map(|_| ())
        .map_err(|_| ActionError::Invalid)
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("email operation is invalid")]
    Invalid,
    #[error("email operation target was not found")]
    NotFound,
    #[error("email operation precondition failed")]
    Precondition,
    #[error("email operation persistence failed")]
    Database(#[from] sqlx::Error),
    #[error("email operation encryption failed")]
    Encryption(#[from] anyhow::Error),
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.operations.actions.tests.rs"]
mod tests;
