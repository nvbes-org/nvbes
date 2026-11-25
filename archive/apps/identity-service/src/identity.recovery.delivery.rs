use crate::mfa_crypto::MfaCrypto;
use nvbes_email::{EmailClient, EmailClientError, EmailCommand, EmailReceipt};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Default, serde::Serialize)]
pub struct BatchResult {
    pub claimed: u32,
    pub accepted: u32,
    pub deferred: u32,
    pub failed: u32,
}

/// Issue the challenge, audit and encrypted delivery atomically. No token is returned.
pub async fn enqueue(
    db: &PgPool,
    crypto: &MfaCrypto,
    base_url: &str,
    email: &str,
) -> anyhow::Result<Uuid> {
    let mut tx = db.begin().await?;
    let recovery = crate::recovery::issue(&mut tx, email).await?;
    let command = crate::email::recovery_command(base_url, &recovery)?;
    let plaintext = serde_json::to_string(&command)?;
    anyhow::ensure!(plaintext.len() <= 16384, "recovery command too large");
    let sealed = crypto.seal_recovery(recovery.principal_id, recovery.challenge_id, &plaintext)?;
    sqlx::query("INSERT INTO identity_recovery_deliveries(challenge_id,principal_id,ciphertext,nonce,key_version,deliver_before) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(recovery.challenge_id).bind(recovery.principal_id).bind(sealed.ciphertext)
        .bind(sealed.nonce.as_slice()).bind(sealed.key_version).bind(recovery.expires_at)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(recovery.challenge_id)
}

pub async fn run_batch(
    db: &PgPool,
    crypto: &MfaCrypto,
    client: &EmailClient,
) -> anyhow::Result<BatchResult> {
    run_with(db, crypto, |command| client.send(command)).await
}

#[derive(sqlx::FromRow)]
struct Delivery {
    challenge_id: Uuid,
    principal_id: Uuid,
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    key_version: i16,
    attempts: i16,
    deliver_before: chrono::DateTime<chrono::Utc>,
}

async fn run_with<F, Fut>(db: &PgPool, crypto: &MfaCrypto, send: F) -> anyhow::Result<BatchResult>
where
    F: Fn(EmailCommand) -> Fut,
    Fut: std::future::Future<Output = Result<EmailReceipt, EmailClientError>>,
{
    sqlx::query("DELETE FROM identity_recovery_deliveries WHERE challenge_id IN (SELECT challenge_id FROM identity_recovery_deliveries WHERE settled_at<clock_timestamp()-interval '30 days' ORDER BY settled_at LIMIT 32 FOR UPDATE SKIP LOCKED)")
        .execute(db).await?;
    // Invalidated links and exhausted leases never reach the Email service.
    sqlx::query("UPDATE identity_recovery_deliveries d SET state=CASE WHEN d.deliver_before<=clock_timestamp() THEN 'expired' ELSE 'cancelled' END,outcome='deadline_or_invalidated',ciphertext=NULL,nonce=NULL,key_version=NULL,lease_token=NULL,lease_expires_at=NULL,settled_at=clock_timestamp() WHERE challenge_id IN (SELECT q.challenge_id FROM identity_recovery_deliveries q JOIN identity_recovery_challenges r ON r.id=q.challenge_id JOIN identity_principals p ON p.id=q.principal_id WHERE q.state IN ('pending','sending') AND (q.deliver_before<=clock_timestamp() OR r.consumed_at IS NOT NULL OR p.status<>'active' OR (q.attempts=8 AND q.lease_expires_at<=clock_timestamp())) ORDER BY q.deliver_before LIMIT 32 FOR UPDATE OF q SKIP LOCKED)")
        .execute(db).await?;
    let mut result = BatchResult::default();
    for _ in 0..16 {
        let lease = Uuid::new_v4();
        let row: Option<Delivery> = sqlx::query_as("UPDATE identity_recovery_deliveries SET state='sending',attempts=attempts+1,lease_token=$1,lease_expires_at=clock_timestamp()+interval '60 seconds' WHERE challenge_id=(SELECT q.challenge_id FROM identity_recovery_deliveries q JOIN identity_recovery_challenges r ON r.id=q.challenge_id JOIN identity_principals p ON p.id=q.principal_id WHERE ((q.state='pending' AND q.available_at<=clock_timestamp()) OR (q.state='sending' AND q.lease_expires_at<=clock_timestamp())) AND q.deliver_before>clock_timestamp() AND r.expires_at>clock_timestamp() AND r.consumed_at IS NULL AND p.status='active' AND q.attempts<8 ORDER BY q.available_at LIMIT 1 FOR UPDATE OF q SKIP LOCKED) RETURNING challenge_id,principal_id,ciphertext,nonce,key_version,attempts,deliver_before")
            .bind(lease).fetch_optional(db).await?;
        let Some(row) = row else {
            break;
        };
        result.claimed += 1;
        let prepared = prepare(db, crypto, &row).await;
        let sent = match prepared {
            Ok(command) => {
                match tokio::time::timeout(std::time::Duration::from_secs(20), send(command)).await
                {
                    Ok(Ok(receipt))
                        if receipt.deliver_before == row.deliver_before
                            && receipt.accepted_at < row.deliver_before
                            && !receipt.message_id.is_empty()
                            && receipt.message_id.len() <= 200 =>
                    {
                        Ok(())
                    }
                    Ok(Ok(_)) => Err(EmailClientError::Protocol),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(EmailClientError::Unavailable),
                }
            }
            Err(error) => Err(error),
        };
        let (state, outcome) = match sent {
            Ok(()) => ("accepted", "email_accepted"),
            Err(EmailClientError::Unavailable) if row.attempts < 8 => {
                ("pending", "email_unavailable")
            }
            Err(EmailClientError::Unavailable) => ("failed", "attempts_exhausted"),
            Err(EmailClientError::Unauthorized) => ("failed", "unauthorized"),
            Err(EmailClientError::Conflict) => ("failed", "idempotency_conflict"),
            Err(_) => ("failed", "invalid_command_or_receipt"),
        };
        let backoff = 30_i32 * (1_i32 << (row.attempts - 1).min(6));
        let changed = sqlx::query("UPDATE identity_recovery_deliveries SET state=$3,outcome=$4,ciphertext=CASE WHEN $3='pending' THEN ciphertext ELSE NULL END,nonce=CASE WHEN $3='pending' THEN nonce ELSE NULL END,key_version=CASE WHEN $3='pending' THEN key_version ELSE NULL END,available_at=clock_timestamp()+make_interval(secs=>$5::double precision),lease_token=NULL,lease_expires_at=NULL,settled_at=CASE WHEN $3='pending' THEN NULL ELSE clock_timestamp() END WHERE challenge_id=$1 AND state='sending' AND lease_token=$2")
            .bind(row.challenge_id).bind(lease).bind(state).bind(outcome).bind(backoff)
            .execute(db).await?.rows_affected();
        if changed == 1 {
            match state {
                "accepted" => result.accepted += 1,
                "pending" => result.deferred += 1,
                _ => result.failed += 1,
            }
        }
    }
    Ok(result)
}

async fn prepare(
    db: &PgPool,
    crypto: &MfaCrypto,
    row: &Delivery,
) -> Result<EmailCommand, EmailClientError> {
    let plaintext = crypto
        .open_recovery(
            row.principal_id,
            row.challenge_id,
            row.key_version,
            &row.ciphertext,
            &row.nonce,
        )
        .map_err(|_| EmailClientError::Protocol)?;
    let command: EmailCommand =
        serde_json::from_str(&plaintext).map_err(|_| EmailClientError::Protocol)?;
    if command.deliver_before != row.deliver_before || command.validate(chrono::Utc::now()).is_err()
    {
        return Err(EmailClientError::Protocol);
    }
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_recovery_challenges r JOIN identity_principals p ON p.id=r.principal_id JOIN identity_login_identifiers i ON i.principal_id=p.id WHERE r.id=$1 AND r.principal_id=$2 AND r.consumed_at IS NULL AND r.expires_at>clock_timestamp() AND p.status='active' AND i.kind='email' AND i.verified_at IS NOT NULL AND i.normalized_value=$3)")
        .bind(row.challenge_id).bind(row.principal_id).bind(&command.recipient.email).fetch_one(db).await
        .map_err(|_| EmailClientError::Unavailable)?;
    if !valid {
        return Err(EmailClientError::Protocol);
    }
    Ok(command)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.recovery.delivery.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.recovery.delivery.runtime.tests.rs"]
mod runtime_tests;
