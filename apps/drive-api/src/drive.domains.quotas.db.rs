use sqlx::Row;
use uuid::Uuid;

use super::logic::BYTES_PER_GB;
use super::types::UsageSnapshot;
use crate::http::error::AppError;

pub async fn ensure_quota_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO quota_usage (workspace_id)
        SELECT id
        FROM workspaces
        WHERE id = $1
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn fetch_quota_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<Option<sqlx::postgres::PgRow>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          w.id AS workspace_id,
          (p.included_storage_gb::bigint * $2::bigint)::bigint AS included_storage_bytes,
          qu.used_storage_bytes,
          qu.file_count,
          COALESCE(monthly_bandwidth.quantity, 0)::bigint AS bandwidth_out_bytes_month,
          qu.updated_at
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN quota_usage qu ON qu.workspace_id = w.id
        LEFT JOIN LATERAL (
          SELECT SUM(quantity)::bigint AS quantity
          FROM usage_events ue
          WHERE ue.workspace_id = w.id
            AND ue.meter = 'bandwidth_out_bytes'
            AND ue.occurred_at >= date_trunc('month', NOW())
        ) monthly_bandwidth ON TRUE
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(BYTES_PER_GB)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

pub async fn lock_usage_snapshot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<UsageSnapshot, AppError> {
    ensure_quota_row(tx, workspace_id).await?;

    let row = sqlx::query(
        r#"
        SELECT
          (p.included_storage_gb::bigint * $2::bigint)::bigint AS included_storage_bytes,
          qu.used_storage_bytes
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN quota_usage qu ON qu.workspace_id = w.id
        WHERE w.id = $1
        FOR UPDATE OF qu
        "#,
    )
    .bind(workspace_id)
    .bind(BYTES_PER_GB)
    .fetch_optional(&mut **tx)
    .await?;

    let row =
        row.ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;

    Ok(UsageSnapshot {
        included_storage_bytes: row.get("included_storage_bytes"),
        used_storage_bytes: row.get("used_storage_bytes"),
    })
}

pub async fn increment_storage_usage(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    size_bytes: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE quota_usage
        SET used_storage_bytes = used_storage_bytes + $2,
            file_count = file_count + 1,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(size_bytes)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn decrement_storage_usage(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    released_storage_bytes: i64,
    deleted_file_count: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE quota_usage
        SET used_storage_bytes = GREATEST(0, used_storage_bytes - $2),
            file_count = GREATEST(0, file_count - $3),
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(released_storage_bytes)
    .bind(deleted_file_count)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn update_bandwidth_usage_cache(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE quota_usage
        SET bandwidth_out_bytes_month = COALESCE((
              SELECT SUM(quantity)::bigint
              FROM usage_events ue
              WHERE ue.workspace_id = $1
                AND ue.meter = 'bandwidth_out_bytes'
                AND ue.occurred_at >= date_trunc('month', NOW())
            ), 0),
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn insert_usage_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    meter: &'static str,
    quantity: i64,
    unit: &'static str,
    source: &str,
    idempotency_key: Option<String>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO usage_events (
          workspace_id,
          meter,
          quantity,
          unit,
          occurred_at,
          source,
          idempotency_key
        )
        VALUES ($1, $2, $3, $4, NOW(), $5, $6)
        ON CONFLICT (workspace_id, idempotency_key)
        WHERE idempotency_key IS NOT NULL
        DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(meter)
    .bind(quantity)
    .bind(unit)
    .bind(source)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
