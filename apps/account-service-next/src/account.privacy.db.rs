use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub const EVENT_TYPE: &str = nvbes_product_account::export_event::ACCOUNT_EXPORT_REQUESTED_V1;

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExportRequest {
    pub export_id: Uuid,
    pub status: String,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExportStatus {
    pub export_id: Uuid,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub participants: Vec<ExportParticipantStatus>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ExportParticipantStatus {
    pub participant: String,
    pub status: String,
    pub attempts: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

pub async fn request(db: &PgPool, principal_id: Uuid) -> Result<ExportRequest, AppError> {
    let export_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let requested_at = Utc::now();
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;

    let closing: bool = sqlx::query_scalar(
        r#"SELECT EXISTS (
          SELECT 1 FROM account_closure_sagas
          WHERE principal_id = $1 AND status IN ('pending', 'dispatching', 'completed')
        )"#,
    )
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;
    if closing {
        return Err(AppError::conflict(
            "account_closure_in_progress",
            "This Account is closing and cannot start a privacy export.",
        ));
    }

    if let Some(existing) = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        r#"
        SELECT id, status, requested_at FROM account_privacy_exports
        WHERE principal_id = $1
          AND (status IN ('pending', 'processing') OR (status = 'completed' AND expires_at > NOW()))
        ORDER BY requested_at DESC LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.commit().await?;
        return Ok(ExportRequest {
            export_id: existing.0,
            status: existing.1,
            requested_at: existing.2,
        });
    }

    sqlx::query(
        "DELETE FROM account_privacy_exports WHERE principal_id = $1 AND expires_at <= NOW()",
    )
    .bind(principal_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"INSERT INTO account_privacy_exports (id, principal_id, status, requested_at, updated_at)
           VALUES ($1, $2, 'pending', $3, $3)"#,
    )
    .bind(export_id)
    .bind(principal_id)
    .bind(requested_at)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"INSERT INTO account_export_participants (export_id, participant, ordinal) VALUES
           ($1, 'cloud', 10), ($1, 'billing', 20),
           ($1, 'identity', 30), ($1, 'account', 40)"#,
    )
    .bind(export_id)
    .execute(&mut *tx)
    .await?;

    let payload = serde_json::to_value(
        nvbes_product_account::export_event::AccountExportRequestedV1::new(
            event_id,
            export_id,
            principal_id,
            requested_at,
        ),
    )
    .map_err(|error| AppError::internal("account_export_event_invalid", error.to_string()))?;
    sqlx::query(
        r#"INSERT INTO account_outbox_events
           (id, aggregate_type, aggregate_id, event_type, payload, occurred_at)
           VALUES ($1, 'account_export', $2, $3, $4, $5)"#,
    )
    .bind(event_id)
    .bind(export_id)
    .bind(EVENT_TYPE)
    .bind(payload)
    .bind(requested_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ExportRequest {
        export_id,
        status: "pending".to_string(),
        requested_at,
    })
}

pub async fn latest(db: &PgPool, principal_id: Uuid) -> Result<ExportStatus, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
            Option<String>,
        ),
    >(
        r#"SELECT id,
                  CASE WHEN status = 'completed' AND expires_at <= NOW() THEN 'expired' ELSE status END,
                  requested_at, updated_at, completed_at, expires_at, last_error
           FROM account_privacy_exports WHERE principal_id = $1
           ORDER BY requested_at DESC, id DESC LIMIT 1"#,
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("account_export_not_found", "No Account export exists."))?;
    let participants = sqlx::query_as::<_, ExportParticipantStatus>(
        r#"SELECT participant, status, attempts, completed_at, last_error
           FROM account_export_participants WHERE export_id = $1 ORDER BY ordinal"#,
    )
    .bind(row.0)
    .fetch_all(db)
    .await?;
    Ok(ExportStatus {
        export_id: row.0,
        status: row.1,
        requested_at: row.2,
        updated_at: row.3,
        completed_at: row.4,
        expires_at: row.5,
        last_error: row.6,
        participants,
    })
}

pub async fn document(db: &PgPool, principal_id: Uuid, export_id: Uuid) -> Result<Value, AppError> {
    sqlx::query_scalar(
        r#"SELECT document FROM account_privacy_exports
           WHERE id = $1 AND principal_id = $2 AND status = 'completed' AND expires_at > NOW()"#,
    )
    .bind(export_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::not_found(
            "account_export_document_not_found",
            "The Account export document is unavailable or expired.",
        )
    })
}
