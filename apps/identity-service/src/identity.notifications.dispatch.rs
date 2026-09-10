use nvbes_email::{EmailClient, EmailClientError, EmailCommand, EmailReceipt};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("notification storage unavailable")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Default, serde::Serialize)]
pub struct BatchResult {
    pub claimed: u32,
    pub accepted: u32,
    pub deferred: u32,
    pub failed: u32,
}

/// One bounded batch, no autonomous loop or external scheduler implied.
pub async fn run_batch(db: &PgPool, client: &EmailClient) -> Result<BatchResult, DispatchError> {
    run_with(db, |command| client.send(command)).await
}

async fn run_with<F, Fut>(db: &PgPool, send: F) -> Result<BatchResult, DispatchError>
where
    F: Fn(EmailCommand) -> Fut,
    Fut: std::future::Future<Output = Result<EmailReceipt, EmailClientError>>,
{
    // Metadata only after settlement; retain it for 30 days, with bounded cleanup.
    sqlx::query("DELETE FROM identity_security_notifications WHERE id IN (SELECT id FROM identity_security_notifications WHERE settled_at<clock_timestamp()-interval '30 days' ORDER BY settled_at LIMIT 32 FOR UPDATE SKIP LOCKED)")
        .execute(db).await?;
    // Expire a bounded number without ever treating an expired attempt as accepted.
    sqlx::query("UPDATE identity_security_notifications SET state=CASE WHEN deliver_before<=clock_timestamp() THEN 'expired' ELSE 'failed' END,command=NULL,lease_token=NULL,lease_expires_at=NULL,settled_at=clock_timestamp(),outcome=CASE WHEN deliver_before<=clock_timestamp() THEN 'deadline' ELSE 'attempts_exhausted' END WHERE id IN (SELECT id FROM identity_security_notifications WHERE state IN ('pending','sending') AND (deliver_before<=clock_timestamp() OR (attempts=8 AND lease_expires_at<=clock_timestamp())) ORDER BY deliver_before LIMIT 32 FOR UPDATE SKIP LOCKED)")
        .execute(db).await?;
    let mut result = BatchResult::default();
    for _ in 0..16 {
        let lease = Uuid::new_v4();
        let row: Option<(Uuid, serde_json::Value, i16)> = sqlx::query_as(
            "UPDATE identity_security_notifications SET state='sending',attempts=attempts+1,lease_token=$1,lease_expires_at=clock_timestamp()+interval '60 seconds' WHERE id=(SELECT id FROM identity_security_notifications WHERE ((state='pending' AND available_at<=clock_timestamp()) OR (state='sending' AND lease_expires_at<=clock_timestamp())) AND deliver_before>clock_timestamp() AND attempts<8 ORDER BY available_at,created_at LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING id,command,attempts",
        ).bind(lease).fetch_optional(db).await?;
        let Some((id, value, attempts)) = row else {
            break;
        };
        result.claimed += 1;
        let command: Result<EmailCommand, _> = serde_json::from_value(value);
        let sent = match command {
            Ok(command) => {
                let deadline = command.deliver_before;
                match tokio::time::timeout(std::time::Duration::from_secs(20), send(command)).await
                {
                    Ok(Ok(receipt))
                        if receipt.deliver_before == deadline
                            && !receipt.message_id.is_empty()
                            && receipt.message_id.len() <= 200
                            && receipt.accepted_at < deadline =>
                    {
                        Ok(receipt)
                    }
                    Ok(Ok(_)) => Err(EmailClientError::Protocol),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(EmailClientError::Unavailable),
                }
            }
            Err(_) => Err(EmailClientError::Protocol),
        };
        let accepted_at = sent.as_ref().ok().map(|receipt| receipt.accepted_at);
        let (state, outcome, receipt) = match sent {
            Ok(receipt) => ("accepted", "email_accepted", Some(receipt.message_id)),
            Err(EmailClientError::Unavailable) if attempts < 8 => {
                ("pending", "email_unavailable", None)
            }
            Err(EmailClientError::Unavailable) => ("failed", "attempts_exhausted", None),
            Err(EmailClientError::Conflict) => ("failed", "idempotency_conflict", None),
            Err(EmailClientError::Unauthorized) => ("failed", "unauthorized", None),
            Err(_) => ("failed", "invalid_command_or_receipt", None),
        };
        let backoff = 30_i32 * (1_i32 << (attempts - 1).min(6));
        let updated = sqlx::query("UPDATE identity_security_notifications SET state=$3,outcome=$4,receipt_id=$5,email_accepted_at=$7,command=CASE WHEN $3='pending' THEN command ELSE NULL END,available_at=clock_timestamp()+make_interval(secs=>$6::double precision),lease_token=NULL,lease_expires_at=NULL,settled_at=CASE WHEN $3='pending' THEN NULL ELSE clock_timestamp() END WHERE id=$1 AND state='sending' AND lease_token=$2")
            .bind(id).bind(lease).bind(state).bind(outcome).bind(receipt).bind(backoff).bind(accepted_at).execute(db).await?.rows_affected();
        if updated == 1 {
            match state {
                "accepted" => result.accepted += 1,
                "pending" => result.deferred += 1,
                _ => result.failed += 1,
            }
        }
    }
    Ok(result)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.notifications.dispatch.tests.rs"]
mod tests;
