use nvbes_product_account::{
    AccountError,
    email::db::{
        mark_email_event_processed_tx, suppress_email_tx,
        update_email_message_status_by_provider_id_tx,
    },
};
use serde_json::Value;
use sqlx::PgPool;

pub(super) async fn process_email_event(db: &PgPool, payload: &Value) -> Result<(), AccountError> {
    let provider_event_id = payload
        .get("provider_event_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AccountError::bad_request("invalid_event_payload", "Missing provider_event_id")
        })?;
    let provider_email_id = payload
        .get("provider_email_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let email = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| AccountError::bad_request("invalid_event_payload", "Missing email"))?;
    let event_type = payload
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| AccountError::bad_request("invalid_event_payload", "Missing event_type"))?;

    let msg_status = match event_type {
        "email_delivered" => Some("delivered"),
        "email_mailbox_not_found" => Some("bounced"),
        "email_bounced" | "email_blacklisted" => Some("bounced"),
        "email_spam" => Some("complained"),
        "email_dropped" => Some("dropped"),
        _ => None,
    };

    let mut tx = db.begin().await?;

    if let Some(status) = msg_status
        && !provider_email_id.is_empty()
    {
        update_email_message_status_by_provider_id_tx(&mut tx, provider_email_id, status).await?;
    }

    match event_type {
        "email_mailbox_not_found" => {
            suppress_email_tx(
                &mut tx,
                email,
                "hard_bounce",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_spam" | "email_bounced" | "email_blacklisted" => {
            suppress_email_tx(
                &mut tx,
                email,
                if event_type == "email_spam" {
                    "spam_complaint"
                } else {
                    "bounced"
                },
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_unsubscribed" => {
            suppress_email_tx(
                &mut tx,
                email,
                "unsubscribed",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_dropped" => {
            suppress_email_tx(
                &mut tx,
                email,
                "dropped",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        _ => {}
    }

    mark_email_event_processed_tx(&mut tx, provider_event_id).await?;
    tx.commit().await?;

    Ok(())
}
