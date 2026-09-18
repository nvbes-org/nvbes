use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    app::BillingState, audit::record_audit_event, error::BillingError,
    reconciliation::record_reconciliation_item,
    subscriptions::apply_subscription_event_on_connection,
};

pub async fn stripe_webhook_handler(
    State(state): State<BillingState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, BillingError> {
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok());

    // 1. Verify signature with timestamp tolerance
    if let Err(err) = nvbes_billing::stripe::verify_stripe_signature(
        &state.config.stripe_webhook_secret,
        sig_header,
        &body,
    ) {
        return Err(BillingError::Invalid(err));
    }

    // 2. Parse event
    let event = nvbes_billing::stripe::parse_stripe_event(&body)
        .ok_or(BillingError::Invalid("malformed_stripe_webhook_payload"))?;

    // 3. Reject livemode webhooks
    if event.livemode {
        return Err(BillingError::LiveModeRejected(
            "Stripe livemode webhooks are strictly forbidden in V1",
        ));
    }

    let raw_json: Value =
        serde_json::from_slice(&body).map_err(|_| BillingError::Invalid("invalid_json"))?;

    let event_created = raw_json
        .get("created")
        .and_then(Value::as_i64)
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
        .unwrap_or_else(Utc::now);

    // Serialize duplicate deliveries and commit effects with the receipt.
    // A failed transaction rolls back all effects, allowing Stripe to retry.
    let mut transaction = state.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(&event.id)
        .execute(&mut *transaction)
        .await?;
    let completed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM billing_webhook_events WHERE event_id = $1 AND processed_at IS NOT NULL AND status IN ('processed', 'ignored'))"
    ).bind(&event.id).fetch_one(&mut *transaction).await?;
    if completed {
        transaction.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(json!({ "received": true, "idempotent": true })),
        ));
    }
    let inserted: Option<String> = sqlx::query_scalar(
        r#"
        INSERT INTO billing_webhook_events (event_id, event_type, stripe_created_at, payload, status)
        VALUES ($1, $2, $3, $4, 'processed')
        ON CONFLICT (event_id) DO UPDATE SET status = 'processed', error_message = NULL, processed_at = NULL
        RETURNING event_id
        "#,
    )
    .bind(&event.id)
    .bind(&event.event_type)
    .bind(event_created)
    .bind(&raw_json)
    .fetch_optional(&mut *transaction)
    .await?;

    if inserted.is_none() {
        // Idempotent duplicate: already received and processed
        return Ok((
            StatusCode::OK,
            Json(json!({ "received": true, "idempotent": true })),
        ));
    }

    // 5. Process event
    match process_stripe_event(
        &mut transaction,
        &event.id,
        &event.event_type,
        event_created,
        &event.data_object,
    )
    .await
    {
        Ok(_) => {
            sqlx::query("UPDATE billing_webhook_events SET processed_at = clock_timestamp() WHERE event_id = $1")
                .bind(&event.id)
                .execute(&mut *transaction)
                .await?;
            transaction.commit().await?;

            let _ = crate::outbox::publish_pending_outbox_events(
                &state.db,
                state.email_client(),
                &state.config.app_url,
                10,
            )
            .await;

            Ok((StatusCode::OK, Json(json!({ "received": true }))))
        }
        Err(err) => {
            transaction.rollback().await?;
            // Record failure in dead-letter / reconciliation queue
            let error_msg = err.to_string();
            let _ = sqlx::query("INSERT INTO billing_webhook_events (event_id, event_type, stripe_created_at, payload, status, error_message) VALUES ($1, $3, $4, $5, 'failed', $2) ON CONFLICT (event_id) DO NOTHING")
                .bind(&event.id)
                .bind(&error_msg)
                .bind(&event.event_type)
                .bind(event_created)
                .bind(&raw_json)
                .execute(&state.db)
                .await;

            let _ = record_reconciliation_item(
                &state.db,
                Some(&event.id),
                None,
                "webhook_processing_failed",
                &json!({ "event_type": event.event_type, "error": error_msg }),
            )
            .await;

            Err(err)
        }
    }
}

async fn process_stripe_event(
    db: &mut sqlx::PgConnection,
    event_id: &str,
    event_type: &str,
    created: DateTime<Utc>,
    data: &Value,
) -> Result<(), BillingError> {
    match event_type {
        "checkout.session.completed" => {
            if let Some(session_id) = data.get("id").and_then(Value::as_str) {
                sqlx::query("UPDATE billing_checkout_sessions SET status = 'completed', completed_at = clock_timestamp() WHERE stripe_session_id = $1")
                    .bind(session_id)
                    .execute(&mut *db)
                    .await?;

                let account_id: Option<Uuid> = sqlx::query_scalar(
                    "SELECT account_id FROM billing_checkout_sessions WHERE stripe_session_id = $1",
                )
                .bind(session_id)
                .fetch_optional(&mut *db)
                .await?;

                if let Some(acc_id) = account_id {
                    record_audit_event(
                        &mut *db,
                        acc_id,
                        "stripe",
                        "checkout_completed",
                        &json!({ "session_id": session_id }),
                    )
                    .await?;
                }
            }
        }
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted"
        | "invoice.paid"
        | "invoice.payment_failed" => {
            apply_subscription_event_on_connection(db, event_id, event_type, created, data).await?;
        }
        _ => {
            // Unhandled events are safely ignored
        }
    }
    Ok(())
}
