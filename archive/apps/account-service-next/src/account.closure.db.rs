use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub const EVENT_TYPE: &str = nvbes_product_account::closure_event::ACCOUNT_CLOSURE_REQUESTED_V1;

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ClosureRequest {
    pub saga_id: Uuid,
    pub status: String,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ClosureStatus {
    pub saga_id: Uuid,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub participants: Vec<ClosureParticipantStatus>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ClosureParticipantStatus {
    pub participant: String,
    pub status: String,
    pub attempts: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

pub async fn latest(db: &PgPool, principal_id: Uuid) -> Result<ClosureStatus, AppError> {
    let saga = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<String>,
        ),
    >(
        r#"
        SELECT id, status, requested_at, updated_at, completed_at, last_error
        FROM account_closure_sagas
        WHERE principal_id = $1
        ORDER BY requested_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::not_found("account_closure_not_found", "No Account closure exists.")
    })?;
    let participants = sqlx::query_as::<_, ClosureParticipantStatus>(
        r#"
        SELECT participant, status, attempts, completed_at, last_error
        FROM account_closure_participants
        WHERE saga_id = $1
        ORDER BY ordinal
        "#,
    )
    .bind(saga.0)
    .fetch_all(db)
    .await?;
    Ok(ClosureStatus {
        saga_id: saga.0,
        status: saga.1,
        requested_at: saga.2,
        updated_at: saga.3,
        completed_at: saga.4,
        last_error: saga.5,
        participants,
    })
}

pub async fn request(db: &PgPool, principal_id: Uuid) -> Result<ClosureRequest, AppError> {
    let saga_id = Uuid::new_v4();
    let requested_at = Utc::now();
    let mut transaction = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(principal_id)
        .execute(&mut *transaction)
        .await?;

    let existing = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        r#"
        SELECT id, status, requested_at
        FROM account_closure_sagas
        WHERE principal_id = $1 AND status IN ('pending', 'dispatching', 'completed')
        ORDER BY requested_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(&mut *transaction)
    .await?;
    if let Some((saga_id, status, requested_at)) = existing {
        transaction.commit().await?;
        return Ok(ClosureRequest {
            saga_id,
            status,
            requested_at,
        });
    }

    sqlx::query(
        r#"
        INSERT INTO account_closure_sagas
          (id, principal_id, status, requested_at, updated_at)
        VALUES ($1, $2, 'pending', $3, $3)
        "#,
    )
    .bind(saga_id)
    .bind(principal_id)
    .bind(requested_at)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO account_closure_participants (saga_id, participant, ordinal)
        VALUES
          ($1, 'cloud', 10),
          ($1, 'billing', 20),
          ($1, 'identity', 30),
          ($1, 'account', 40)
        "#,
    )
    .bind(saga_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET dead_lettered_at = NOW(),
            claimed_at = NULL,
            last_error_code = 'account_closure_requested',
            last_error_summary = 'Superseded by account closure'
        WHERE aggregate_id = $1
          AND event_type = 'account.oidc-profile.updated.v1'
          AND published_at IS NULL
          AND dead_lettered_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        UPDATE account_outbox_events event
        SET dead_lettered_at = NOW(), claimed_at = NULL,
            last_error_code = 'account_closure_requested',
            last_error_summary = 'Superseded by account closure'
        FROM account_privacy_exports export
        WHERE event.aggregate_id = export.id
          AND export.principal_id = $1
          AND event.event_type = 'account.export.requested.v1'
          AND event.published_at IS NULL AND event.dead_lettered_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        UPDATE account_privacy_exports
        SET status = 'failed', updated_at = NOW(), last_error = 'account_closure_requested'
        WHERE principal_id = $1 AND status IN ('pending', 'processing')
        "#,
    )
    .bind(principal_id)
    .execute(&mut *transaction)
    .await?;

    let event_id = Uuid::new_v4();
    let payload = event_payload(event_id, saga_id, principal_id, requested_at)?;
    sqlx::query(
        r#"
        INSERT INTO account_outbox_events
          (id, aggregate_type, aggregate_id, event_type, payload, occurred_at)
        VALUES ($1, 'account_closure', $2, $3, $4, $5)
        "#,
    )
    .bind(event_id)
    .bind(saga_id)
    .bind(EVENT_TYPE)
    .bind(payload)
    .bind(requested_at)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(ClosureRequest {
        saga_id,
        status: "pending".to_string(),
        requested_at,
    })
}

fn event_payload(
    event_id: Uuid,
    saga_id: Uuid,
    principal_id: Uuid,
    requested_at: DateTime<Utc>,
) -> Result<Value, AppError> {
    serde_json::to_value(
        nvbes_product_account::closure_event::AccountClosureRequestedV1::new(
            event_id,
            saga_id,
            principal_id,
            requested_at,
        ),
    )
    .map_err(|error| {
        AppError::internal(
            "account_closure_event_invalid",
            format!("Account closure event could not be serialized: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::{EVENT_TYPE, event_payload};

    #[test]
    fn closure_event_identifies_saga_subject_and_event() {
        let event = Uuid::new_v4();
        let saga = Uuid::new_v4();
        let principal = Uuid::new_v4();
        let requested_at = Utc
            .with_ymd_and_hms(2026, 7, 30, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let payload = event_payload(event, saga, principal, requested_at).expect("closure event");
        assert_eq!(payload["event_id"], event.to_string());
        assert_eq!(payload["event_type"], EVENT_TYPE);
        assert_eq!(payload["saga_id"], saga.to_string());
        assert_eq!(payload["principal_id"], principal.to_string());
    }
}
