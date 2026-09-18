use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::{
    email::{
        BillingPaymentFailureNotification, BillingReceiptNotification, deliver_billing_receipt,
        deliver_payment_failure,
    },
    state::BillingWorkerState,
};

#[derive(Debug, PartialEq, Eq)]
pub enum DispatchOutcome {
    Acknowledged,
    Retry,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OutboxRow {
    pub id: i64,
    pub event_uuid: Uuid,
    pub event_type: String,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn dispatch_message(
    state: &BillingWorkerState,
    identifier: &str,
) -> anyhow::Result<DispatchOutcome> {
    let trimmed = identifier.trim();
    if trimmed.is_empty() {
        return Ok(DispatchOutcome::Acknowledged);
    }

    let mut tx = state.db.begin().await?;

    let event = if let Ok(uuid_val) = Uuid::parse_str(trimmed) {
        sqlx::query_as::<_, OutboxRow>(
            r#"
            SELECT id, event_uuid, event_type, aggregate_id, payload, published_at, created_at
            FROM billing_outbox
            WHERE event_uuid = $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(uuid_val)
        .fetch_optional(&mut *tx)
        .await?
    } else if let Ok(id_val) = trimmed.parse::<i64>() {
        sqlx::query_as::<_, OutboxRow>(
            r#"
            SELECT id, event_uuid, event_type, aggregate_id, payload, published_at, created_at
            FROM billing_outbox
            WHERE id = $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(id_val)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        tracing::warn!(identifier = %trimmed, "unknown billing queue message identifier format");
        return Ok(DispatchOutcome::Acknowledged);
    };

    let Some(event) = event else {
        // Either not found or locked by another worker
        return Ok(DispatchOutcome::Acknowledged);
    };

    if event.published_at.is_some() {
        // Idempotent no-op
        return Ok(DispatchOutcome::Acknowledged);
    }

    if let Err(e) = process_event(state, &event).await {
        tracing::error!(error = ?e, event_id = %event.id, "event processing failed, signaling retry");
        return Ok(DispatchOutcome::Retry);
    }

    sqlx::query("UPDATE billing_outbox SET published_at = clock_timestamp() WHERE id = $1")
        .bind(event.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    metrics::counter!(
        "nvbes_billing_worker_events_total",
        "event_type" => event.event_type,
        "status" => "processed"
    )
    .increment(1);

    Ok(DispatchOutcome::Acknowledged)
}

async fn process_event(state: &BillingWorkerState, event: &OutboxRow) -> anyhow::Result<()> {
    if let Some(client) = state.email_client.as_ref() {
        match event.event_type.as_str() {
            "billing.invoice.paid.v1" => {
                if let Some(email) = event.payload.get("recipient_email").and_then(Value::as_str) {
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
                if let Some(email) = event.payload.get("recipient_email").and_then(Value::as_str) {
                    let default_portal = format!("{}/billing/portal", state.config.app_url);
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
            _ => {
                tracing::debug!(event_type = %event.event_type, "ignoring unhandled event type in billing worker");
            }
        }
    }
    Ok(())
}

pub async fn run_local(
    state: BillingWorkerState,
    mut receiver: mpsc::Receiver<String>,
    mut shutdown: watch::Receiver<bool>,
) {
    tracing::info!("starting local in-memory billing worker loop");
    let mut sweep_interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            Some(msg_id) = receiver.recv() => {
                if let Err(e) = dispatch_message(&state, &msg_id).await {
                    tracing::error!(error = ?e, %msg_id, "local billing dispatch failed");
                }
            }
            _ = sweep_interval.tick() => {
                let _ = sweep_pending(&state).await;
            }
        }
    }
    tracing::info!("local billing worker loop terminated");
}

pub async fn sweep_pending(state: &BillingWorkerState) -> anyhow::Result<usize> {
    let rows = sqlx::query_as::<_, OutboxRow>(
        r#"
        SELECT id, event_uuid, event_type, aggregate_id, payload, published_at, created_at
        FROM billing_outbox
        WHERE published_at IS NULL
        ORDER BY id ASC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let count = rows.len();
    for row in rows {
        let _ = dispatch_message(state, &row.event_uuid.to_string()).await;
    }
    Ok(count)
}
