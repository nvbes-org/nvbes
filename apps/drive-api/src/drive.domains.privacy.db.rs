use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::types::{PrivacyRequestDraft, PrivacyRequestStatusResponse};
use crate::http::error::AppError;

pub async fn fetch_privacy_request(
    pool: &PgPool,
    user_id: Uuid,
    principal_id: Uuid,
    request_id: Uuid,
) -> Result<Option<PrivacyRequestStatusResponse>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          id,
          request_type::text AS request_type,
          status::text AS status,
          worker_job_id,
          requested_by_principal_id,
          rejection_reason,
          result,
          requested_at,
          completed_at
        FROM privacy_requests
        WHERE id = $1
          AND (
            requested_by = $2
            OR requested_by_principal_id = $3
            OR subject_user_id = $2
            OR workspace_id IN (
              SELECT workspace_id
              FROM workspace_memberships
              WHERE user_id = $2
                AND status = 'active'
                AND role IN ('owner', 'admin')
            )
          )
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .bind(principal_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        let result: Option<sqlx::types::Json<Value>> = r.get("result");
        PrivacyRequestStatusResponse {
            request_id: r.get("id"),
            request_type: r.get("request_type"),
            status: r.get("status"),
            worker_job_id: r.get("worker_job_id"),
            requested_by_principal_id: r.get("requested_by_principal_id"),
            rejection_reason: r.get("rejection_reason"),
            result: result.map(|v| v.0),
            requested_at: r.get("requested_at"),
            completed_at: r.get("completed_at"),
        }
    }))
}

pub struct InsertRequestResult {
    pub id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub worker_job_id: Option<Uuid>,
    pub status: String,
    pub requested_by_principal_id: Uuid,
}

pub async fn insert_privacy_request_tx(
    tx: &mut Transaction<'_, Postgres>,
    draft: &PrivacyRequestDraft,
) -> Result<InsertRequestResult, AppError> {
    let row = sqlx::query(
        r#"
        INSERT INTO privacy_requests (
          request_type,
          subject_user_id,
          workspace_id,
          requested_by,
          requested_by_principal_id
        )
        VALUES ($1::privacy_request_type, $2, $3, $4, $5)
        ON CONFLICT (request_type, (COALESCE(subject_user_id, workspace_id)))
          WHERE status IN ('queued', 'processing')
        DO UPDATE SET updated_at = privacy_requests.updated_at
        RETURNING id, requested_at, worker_job_id, status::text AS status
        "#,
    )
    .bind(draft.request_type)
    .bind(draft.subject_user_id)
    .bind(draft.workspace_id)
    .bind(draft.requested_by)
    .bind(draft.requested_by_principal_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(InsertRequestResult {
        id: row.get("id"),
        requested_at: row.get("requested_at"),
        worker_job_id: row.get("worker_job_id"),
        status: row.get("status"),
        requested_by_principal_id: draft.requested_by_principal_id,
    })
}

pub async fn insert_worker_job_tx(
    redis: &nvbes_redis::RedisPool,
    queue: &str,
    job_type: &str,
    idempotency_key: &str,
    payload: Value,
    job_id: Uuid,
) -> Result<Uuid, AppError> {
    Ok(nvbes_redis::worker_queue::enqueue_job(
        redis,
        queue,
        job_type,
        payload,
        Some(idempotency_key),
        3,
        true,
        Some(job_id),
    )
    .await
    .map_err(|error| AppError::internal("redis_worker_queue_enqueue_failed", &error.to_string()))?)
}

pub async fn update_privacy_request_job_tx(
    tx: &mut Transaction<'_, Postgres>,
    request_id: Uuid,
    job_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET worker_job_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(job_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn count_owned_workspaces(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM workspaces
        WHERE (owner_user_id = $1 OR owner_principal_id = $1)
          AND deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn check_legal_hold(pool: &PgPool, workspace_id: Uuid) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM privacy_requests
          WHERE workspace_id = $1
            AND legal_hold = TRUE
            AND status IN ('queued', 'processing')
        )
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}
