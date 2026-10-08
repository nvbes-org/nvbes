use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Runs inside the security mutation transaction: recipients cannot drift between retries.
pub(crate) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    principal: Uuid,
    event_type: &str,
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let recipients: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,normalized_value FROM identity_login_identifiers WHERE principal_id=$1 AND kind='email' AND verified_at IS NOT NULL ORDER BY id LIMIT 17 FOR SHARE",
    ).bind(principal).fetch_all(&mut **tx).await?;
    // Bound work inside the authentication transaction; never silently omit recipients.
    if recipients.len() > 16 {
        return Err(sqlx::Error::Protocol(
            "too many verified security notification recipients".into(),
        ));
    }
    if recipients.is_empty() {
        sqlx::query("INSERT INTO identity_security_notifications(id,event_id,principal_id,state,deliver_before,outcome,settled_at) VALUES($1,$2,$3,'no_recipient',$4+interval '24 hours','verified_email_missing',clock_timestamp())")
            .bind(Uuid::new_v4()).bind(event_id).bind(principal).bind(occurred_at).execute(&mut **tx).await?;
    }
    for (identifier, recipient) in recipients {
        let command = crate::notification_command::recovery_command(
            event_id,
            principal,
            event_type,
            occurred_at,
            recipient,
            occurred_at,
        )
        .map_err(|_| sqlx::Error::Protocol("invalid security notification snapshot".into()))?;
        let value = serde_json::to_value(&command).map_err(|_| {
            sqlx::Error::Protocol("invalid security notification serialization".into())
        })?;
        sqlx::query("INSERT INTO identity_security_notifications(id,event_id,principal_id,recipient_identifier_id,command,state,deliver_before) VALUES($1,$2,$3,$4,$5,'pending',$6)")
            .bind(Uuid::new_v4()).bind(event_id).bind(principal).bind(identifier).bind(value)
            .bind(command.deliver_before).execute(&mut **tx).await?;
    }
    Ok(())
}
