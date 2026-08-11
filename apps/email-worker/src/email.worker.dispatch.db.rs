use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::dispatch_attempt::{
    FailureResolution, failure_resolution, finish_attempt, finish_attempt_tx,
};

#[derive(Debug, FromRow)]
pub struct ClaimedEmail {
    pub id: Uuid,
    pub category: String,
    pub template_name: String,
    pub recipient_ciphertext: Vec<u8>,
    pub recipient_nonce: Vec<u8>,
    pub template_ciphertext: Vec<u8>,
    pub template_nonce: Vec<u8>,
    pub deliver_before: DateTime<Utc>,
    pub message_id: String,
    pub attempt_count: i32,
    pub lease_token: Uuid,
}

#[derive(Debug)]
pub enum ClaimResult {
    Claimed(ClaimedEmail),
    Busy,
    Settled,
    Missing,
}

pub async fn claim_message(pool: &PgPool, id: Uuid) -> Result<ClaimResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, (String, Option<Uuid>, bool, bool, bool)>(
        r#"
        SELECT state::text, lease_token,
               COALESCE(lease_expires_at <= clock_timestamp(), TRUE),
               deliver_before <= clock_timestamp(),
               EXISTS (
                   SELECT 1 FROM email_suppressions AS suppression
                   WHERE suppression.recipient_hash = email_messages.recipient_hash
                     AND suppression.active
                     AND suppression.released_at IS NULL
                     AND (suppression.scope = 'all' OR email_messages.category = 'reminder')
               )
        FROM email_messages
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((state, stale_lease_token, lease_expired, deadline_expired, suppressed)) = row else {
        tx.commit().await?;
        return Ok(ClaimResult::Missing);
    };

    if !matches!(state.as_str(), "accepted" | "deferred" | "dispatching") {
        tx.commit().await?;
        return Ok(ClaimResult::Settled);
    }
    if state == "dispatching" && !lease_expired {
        tx.commit().await?;
        return Ok(ClaimResult::Settled);
    }
    if let Some(stale_lease_token) = stale_lease_token {
        finish_attempt_tx(
            &mut tx,
            stale_lease_token,
            "ambiguous_failure",
            None,
            Some("lease_expired"),
        )
        .await?;
    }
    if deadline_expired || suppressed {
        let terminal_state = if deadline_expired {
            "expired"
        } else {
            "suppressed"
        };
        sqlx::query(
            r#"
            UPDATE email_messages
            SET state = $2::email_message_state, terminal_at = clock_timestamp(),
                updated_at = clock_timestamp(), lease_token = NULL, lease_expires_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(terminal_state)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(ClaimResult::Settled);
    }

    let lease_token = Uuid::new_v4();
    let claimed = sqlx::query_as::<_, ClaimedEmail>(
        r#"
        UPDATE email_messages
        SET state = 'dispatching', lease_token = $2,
            lease_expires_at = clock_timestamp() + interval '45 seconds',
            attempt_count = attempt_count + 1, updated_at = clock_timestamp()
        WHERE id = $1 AND next_attempt_at <= clock_timestamp()
        RETURNING id, category::text AS category, template_name,
                  recipient_ciphertext, recipient_nonce,
                  template_ciphertext, template_nonce, deliver_before, message_id,
                  attempt_count, lease_token
        "#,
    )
    .bind(id)
    .bind(lease_token)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(claimed) = claimed else {
        tx.commit().await?;
        return Ok(ClaimResult::Busy);
    };
    sqlx::query(
        r#"
        INSERT INTO email_delivery_attempts (id, message_id, attempt_number, lease_token)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(claimed.id)
    .bind(claimed.attempt_count)
    .bind(claimed.lease_token)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ClaimResult::Claimed(claimed))
}

pub async fn suppress_due_messages(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE email_messages AS message
        SET state = 'suppressed', terminal_at = clock_timestamp(), updated_at = clock_timestamp()
        FROM email_suppressions AS suppression
        WHERE message.recipient_hash = suppression.recipient_hash
          AND suppression.active
          AND suppression.released_at IS NULL
          AND message.state IN ('accepted', 'deferred')
          AND (suppression.scope = 'all' OR message.category = 'reminder')
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn expire_claim_if_due(pool: &PgPool, claim: &ClaimedEmail) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE email_messages
        SET state = 'expired', terminal_at = clock_timestamp(), updated_at = clock_timestamp(),
            lease_token = NULL, lease_expires_at = NULL
        WHERE id = $1 AND state = 'dispatching' AND lease_token = $2
          AND deliver_before <= clock_timestamp()
        "#,
    )
    .bind(claim.id)
    .bind(claim.lease_token)
    .execute(pool)
    .await?;
    if result.rows_affected() == 1 {
        finish_attempt(
            pool,
            claim.lease_token,
            "expired",
            None,
            Some("delivery_deadline"),
        )
        .await?;
    }
    Ok(result.rows_affected() == 1)
}

pub async fn complete_success(
    pool: &PgPool,
    claim: &ClaimedEmail,
    provider_message_id: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        r#"
        UPDATE email_messages
        SET state = 'provider_accepted', provider_message_id = $3,
            provider_accepted_at = clock_timestamp(), updated_at = clock_timestamp(),
            lease_token = NULL, lease_expires_at = NULL
        WHERE id = $1 AND state = 'dispatching' AND lease_token = $2
        "#,
    )
    .bind(claim.id)
    .bind(claim.lease_token)
    .bind(provider_message_id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 1 {
        finish_attempt_tx(
            &mut tx,
            claim.lease_token,
            "provider_accepted",
            Some(provider_message_id),
            None,
        )
        .await?;
    }
    tx.commit().await
}

pub async fn complete_failure(
    pool: &PgPool,
    claim: &ClaimedEmail,
    outcome: &'static str,
    failure_code: &'static str,
    maximum_attempts: i32,
    retry_delay: Duration,
) -> Result<FailureResolution, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let retry_at = now + retry_delay;
    let resolution = failure_resolution(
        outcome,
        claim.attempt_count,
        maximum_attempts,
        retry_at,
        claim.deliver_before,
    );
    let (state, next_attempt_at, terminal) = match &resolution {
        FailureResolution::Failed => ("failed", now, true),
        FailureResolution::Expired => ("expired", now, true),
        FailureResolution::RetryAt(retry_at) => ("deferred", *retry_at, false),
    };

    let result = sqlx::query(
        r#"
        UPDATE email_messages
        SET state = $3::email_message_state, next_attempt_at = $4,
            terminal_at = CASE WHEN $5 THEN clock_timestamp() ELSE NULL END,
            updated_at = clock_timestamp(), lease_token = NULL, lease_expires_at = NULL
        WHERE id = $1 AND state = 'dispatching' AND lease_token = $2
        "#,
    )
    .bind(claim.id)
    .bind(claim.lease_token)
    .bind(state)
    .bind(next_attempt_at)
    .bind(terminal)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 1 {
        finish_attempt_tx(
            &mut tx,
            claim.lease_token,
            outcome,
            None,
            Some(failure_code),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(resolution)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.dispatch.db.tests.rs"]
mod tests;
