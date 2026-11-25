use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Called by the validated reset transaction, after credential/session mutations.
/// A queue failure must roll back the reset; do not dispatch network calls here.
pub async fn password_recovered(
    tx: &mut Transaction<'_, Postgres>,
    principal: Uuid,
) -> Result<(), sqlx::Error> {
    let event_id = Uuid::new_v4();
    let event_type = "identity.password_recovered";
    let occurred_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO identity_outbox(id,event_type,aggregate_id,payload) VALUES($1,$2,$3,$4) RETURNING occurred_at",
    )
    .bind(event_id)
    .bind(event_type)
    .bind(principal)
    .bind(serde_json::json!({"principal_id":principal}))
    .fetch_one(&mut **tx)
    .await?;
    crate::notification_queue::enqueue(tx, event_id, principal, event_type, occurred_at).await
}
