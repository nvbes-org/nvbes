use chrono::{DateTime, Utc};
use nvbes_email::EmailClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    email::{
        BillingPaymentFailureNotification, BillingReceiptNotification, deliver_billing_receipt,
        deliver_payment_failure,
    },
    error::BillingResult,
};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OutboxEvent {
    pub id: i64,
    pub event_uuid: Uuid,
    pub event_type: String,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn record_outbox_event(
    db: impl PgExecutor<'_>,
    event_type: &str,
    aggregate_id: Uuid,
    payload: &Value,
) -> BillingResult<()> {
    sqlx::query(
        r#"
        INSERT INTO billing_outbox (event_type, aggregate_id, payload)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(event_type)
    .bind(aggregate_id)
    .bind(payload)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn publish_pending_outbox_events(
    pool: &PgPool,
    email_client: Option<&EmailClient>,
    app_url: &str,
    limit: i64,
) -> BillingResult<usize> {
    let mut tx = pool.begin().await?;

    let events = sqlx::query_as::<_, OutboxEvent>(
        r#"
        SELECT id, event_uuid, event_type, aggregate_id, payload, published_at, created_at
        FROM billing_outbox
        WHERE published_at IS NULL
        ORDER BY id ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;

    let count = events.len();
    for event in events {
        if let Some(client) = email_client {
            match event.event_type.as_str() {
                "billing.invoice.paid.v1" => {
                    if let Some(email) =
                        event.payload.get("recipient_email").and_then(Value::as_str)
                    {
                        let notif = BillingReceiptNotification {
                            account_id: event.aggregate_id,
                            recipient_email: email.to_string(),
                            customer_name: event
                                .payload
                                .get("customer_name")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            amount_minor: event
                                .payload
                                .get("amount_minor")
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                            currency: event
                                .payload
                                .get("currency")
                                .and_then(Value::as_str)
                                .unwrap_or("EUR")
                                .to_string(),
                            invoice_id: event
                                .payload
                                .get("invoice_id")
                                .and_then(Value::as_str)
                                .unwrap_or("in_unknown")
                                .to_string(),
                            invoice_url: event
                                .payload
                                .get("invoice_url")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                        };
                        let _ = deliver_billing_receipt(client, notif).await;
                    }
                }
                "billing.invoice.payment_failed.v1" => {
                    if let Some(email) =
                        event.payload.get("recipient_email").and_then(Value::as_str)
                    {
                        let default_portal = format!("{app_url}/billing/portal");
                        let notif = BillingPaymentFailureNotification {
                            account_id: event.aggregate_id,
                            recipient_email: email.to_string(),
                            customer_name: event
                                .payload
                                .get("customer_name")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            amount_minor: event
                                .payload
                                .get("amount_minor")
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                            currency: event
                                .payload
                                .get("currency")
                                .and_then(Value::as_str)
                                .unwrap_or("EUR")
                                .to_string(),
                            invoice_id: event
                                .payload
                                .get("invoice_id")
                                .and_then(Value::as_str)
                                .unwrap_or("in_unknown")
                                .to_string(),
                            billing_portal_url: Some(default_portal),
                            invoice_url: event
                                .payload
                                .get("invoice_url")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                        };
                        let _ = deliver_payment_failure(client, notif).await;
                    }
                }
                _ => {}
            }
        }

        sqlx::query("UPDATE billing_outbox SET published_at = clock_timestamp() WHERE id = $1")
            .bind(event.id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(count)
}
