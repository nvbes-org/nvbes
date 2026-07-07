use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

#[expect(
    clippy::too_many_arguments,
    reason = "Upload session creation persists explicit ownership and checksum metadata."
)]
pub async fn insert_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    created_by: Uuid,
    created_by_principal_id: Uuid,
    expected_size_bytes: i64,
    expected_checksum: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO upload_sessions (
          id,
          workspace_id,
          storage_object_id,
          created_by,
          created_by_principal_id,
          expected_size_bytes,
          expected_checksum,
          status,
          expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7)
        "#,
    )
    .bind(id)
    .bind(workspace_id)
    .bind(storage_object_id)
    .bind(created_by)
    .bind(created_by_principal_id)
    .bind(expected_size_bytes)
    .bind(expected_checksum)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn attach_multipart_upload_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    multipart_upload_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET storage_multipart_upload_id = $3,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(multipart_upload_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn insert_upload_part_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    part_number: i32,
    offset_bytes: i64,
    size_bytes: i64,
    etag: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO upload_parts (
          upload_session_id,
          workspace_id,
          part_number,
          offset_bytes,
          size_bytes,
          etag
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(upload_id)
    .bind(workspace_id)
    .bind(part_number)
    .bind(offset_bytes)
    .bind(size_bytes)
    .bind(etag)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn advance_upload_offset_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    upload_offset_bytes: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET upload_offset_bytes = $3,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(upload_offset_bytes)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn complete_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    completed_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET status = 'completed',
            completed_at = $3,
            updated_at = $3
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(completed_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn cancel_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    cancelled_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET status = 'cancelled',
            updated_at = $3
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(cancelled_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
