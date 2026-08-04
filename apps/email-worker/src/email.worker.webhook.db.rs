use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct TemWebhookEvent {
    pub id: String,
    pub email_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub status: Option<String>,
}

pub struct NotificationResult {
    pub inserted: bool,
    pub processing_result: &'static str,
}

pub async fn record_subscription_confirmation(
    pool: &PgPool,
    sns_message_id: &str,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query(
        r#"
        INSERT INTO email_provider_events (
            id, provider, provider_event_id, sns_message_id, event_type,
            diagnostic, processed_at, processing_result
        ) VALUES ($1, 'scaleway_topics', $2, $2, 'subscription_confirmation',
                  '{}'::jsonb, clock_timestamp(), 'confirmed')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(sns_message_id)
    .execute(pool)
    .await?;
    Ok(inserted.rows_affected() == 1)
}

pub async fn apply_notification(
    pool: &PgPool,
    sns_message_id: &str,
    event: &TemWebhookEvent,
) -> Result<NotificationResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let event_id = Uuid::new_v4();
    let inserted = sqlx::query(
        r#"
        INSERT INTO email_provider_events (
            id, provider, provider_event_id, sns_message_id, provider_message_id,
            event_type, diagnostic
        ) VALUES ($1, 'scaleway_tem', $2, $3, $4, $5, $6)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(&event.id)
    .bind(sns_message_id)
    .bind(&event.email_id)
    .bind(&event.event_type)
    .bind(json!({ "status": bounded_status(event.status.as_deref()) }))
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        tx.commit().await?;
        return Ok(NotificationResult {
            inserted: false,
            processing_result: "duplicate",
        });
    }

    let result = match normalized_state(&event.event_type) {
        Some(state) => apply_transition(&mut tx, &event.email_id, state).await?,
        None => "stored_unknown",
    };
    apply_suppression(&mut tx, event_id, &event.email_id, &event.event_type).await?;
    sqlx::query(
        r#"
        UPDATE email_provider_events
        SET processed_at = clock_timestamp(), processing_result = $2
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .bind(result)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(NotificationResult {
        inserted: true,
        processing_result: result,
    })
}

fn bounded_status(status: Option<&str>) -> Option<String> {
    status.map(|value| value.chars().take(200).collect())
}

fn normalized_state(event_type: &str) -> Option<&'static str> {
    match event_type {
        "email_queued" => Some("provider_accepted"),
        "email_delivered" => Some("delivered"),
        "email_deferred" | "email_soft_bounced" => Some("deferred"),
        "email_spam" => Some("complained"),
        "email_mailbox_not_found" | "email_blocklisted" | "blocklist_created" => {
            Some("hard_bounced")
        }
        "email_dropped" => Some("dropped"),
        _ => None,
    }
}

async fn apply_transition(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider_message_id: &str,
    target: &'static str,
) -> Result<&'static str, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE email_messages
        SET state = $2::email_message_state,
            terminal_at = CASE WHEN $2 IN ('delivered', 'hard_bounced', 'complained', 'dropped')
                               THEN clock_timestamp() ELSE terminal_at END,
            updated_at = clock_timestamp()
        WHERE provider_message_id = $1
          AND (
            ($2 = 'provider_accepted' AND state = 'provider_accepted')
            OR ($2 = 'deferred' AND state IN ('provider_accepted', 'deferred'))
            OR ($2 = 'delivered' AND state IN ('provider_accepted', 'deferred'))
            OR ($2 IN ('hard_bounced', 'complained') AND state IN ('provider_accepted', 'deferred', 'delivered'))
            OR ($2 = 'dropped' AND state IN ('provider_accepted', 'deferred'))
          )
        "#,
    )
    .bind(provider_message_id)
    .bind(target)
    .execute(&mut **tx)
    .await?;
    Ok(if result.rows_affected() == 1 {
        "state_applied"
    } else {
        "state_retained"
    })
}

async fn apply_suppression(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    source_event_id: Uuid,
    provider_message_id: &str,
    event_type: &str,
) -> Result<(), sqlx::Error> {
    if event_type == "email_deferred" || event_type == "email_soft_bounced" {
        sqlx::query(
            r#"
            INSERT INTO email_suppressions (
                recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
                scope, reason, source_event_id, soft_bounce_count, active
            )
            SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'all',
                   'soft_bounce_threshold', $2, 1, FALSE
            FROM email_messages WHERE provider_message_id = $1
            ON CONFLICT (recipient_hash) DO UPDATE
            SET soft_bounce_count = email_suppressions.soft_bounce_count + 1,
                source_event_id = EXCLUDED.source_event_id,
                active = email_suppressions.soft_bounce_count + 1 >= 3,
                suppressed_at = CASE WHEN email_suppressions.soft_bounce_count + 1 >= 3
                                     THEN clock_timestamp() ELSE email_suppressions.suppressed_at END,
                released_at = CASE WHEN email_suppressions.soft_bounce_count + 1 >= 3
                                   THEN NULL ELSE email_suppressions.released_at END
            WHERE email_suppressions.reason = 'soft_bounce_threshold'
              AND NOT email_suppressions.active
            "#,
        )
        .bind(provider_message_id)
        .bind(source_event_id)
        .execute(&mut **tx)
        .await?;
        return Ok(());
    }

    let reason = match event_type {
        "email_spam" => Some("complaint"),
        "email_mailbox_not_found" | "email_blocklisted" | "blocklist_created" => {
            Some("hard_bounce")
        }
        "email_dropped" => Some("provider_drop"),
        _ => None,
    };
    if let Some(reason) = reason {
        sqlx::query(
            r#"
            INSERT INTO email_suppressions (
                recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
                scope, reason, source_event_id, soft_bounce_count, active
            )
            SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'all', $2, $3, 0, TRUE
            FROM email_messages WHERE provider_message_id = $1
            ON CONFLICT (recipient_hash) DO UPDATE
            SET scope = 'all', reason = EXCLUDED.reason, source_event_id = EXCLUDED.source_event_id,
                active = TRUE,
                suppressed_at = clock_timestamp(), released_at = NULL,
                released_by = NULL, release_reason = NULL
            "#,
        )
        .bind(provider_message_id)
        .bind(reason)
        .bind(source_event_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{bounded_status, normalized_state};

    #[test]
    fn tem_events_have_stable_normalized_states() {
        assert_eq!(normalized_state("email_delivered"), Some("delivered"));
        assert_eq!(normalized_state("email_spam"), Some("complained"));
        assert_eq!(normalized_state("blocklist_created"), Some("hard_bounced"));
        assert_eq!(normalized_state("future_event"), None);
    }

    #[test]
    fn provider_diagnostics_are_bounded() {
        assert_eq!(bounded_status(Some(&"x".repeat(500))).unwrap().len(), 200);
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.webhook.db.tests.rs"]
mod database_tests;
