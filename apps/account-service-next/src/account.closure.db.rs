use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn request(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    let saga_id = Uuid::new_v4();
    let requested_at = Utc::now();
    let mut transaction = db.begin().await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO account_closure_sagas
          (id, principal_id, status, requested_at, updated_at)
        VALUES ($1, $2, 'pending', $3, $3)
        ON CONFLICT (principal_id) WHERE status IN ('pending', 'dispatching')
        DO NOTHING
        RETURNING id
        "#,
    )
    .bind(saga_id)
    .bind(principal_id)
    .bind(requested_at)
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(inserted_saga_id) = inserted {
        let event_id = Uuid::new_v4();
        let payload = event_payload(event_id, inserted_saga_id, principal_id, requested_at);
        sqlx::query(
            r#"
            INSERT INTO account_outbox_events
              (id, aggregate_type, aggregate_id, event_type, payload, occurred_at)
            VALUES ($1, 'account_closure', $2, 'account.closure.requested', $3, $4)
            "#,
        )
        .bind(event_id)
        .bind(inserted_saga_id)
        .bind(payload)
        .bind(requested_at)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(())
}

fn event_payload(
    event_id: Uuid,
    saga_id: Uuid,
    principal_id: Uuid,
    requested_at: DateTime<Utc>,
) -> Value {
    json!({
        "event_id": event_id,
        "saga_id": saga_id,
        "principal_id": principal_id,
        "requested_at": requested_at,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::event_payload;

    #[test]
    fn closure_event_identifies_saga_subject_and_event() {
        let event = Uuid::new_v4();
        let saga = Uuid::new_v4();
        let principal = Uuid::new_v4();
        let requested_at = Utc
            .with_ymd_and_hms(2026, 7, 30, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let payload = event_payload(event, saga, principal, requested_at);
        assert_eq!(payload["event_id"], event.to_string());
        assert_eq!(payload["saga_id"], saga.to_string());
        assert_eq!(payload["principal_id"], principal.to_string());
    }
}
