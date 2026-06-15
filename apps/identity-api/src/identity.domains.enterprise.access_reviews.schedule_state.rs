use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn disable_schedule(
    db: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<bool, AppError> {
    Ok(sqlx::query(
        r#"
        UPDATE access_review_schedules
        SET disabled_at = COALESCE(disabled_at, NOW())
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .execute(db)
    .await?
    .rows_affected()
        > 0)
}

pub async fn enable_schedule(
    db: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<bool, AppError> {
    Ok(sqlx::query(
        r#"
        UPDATE access_review_schedules
        SET disabled_at = NULL
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .execute(db)
    .await?
    .rows_affected()
        > 0)
}
