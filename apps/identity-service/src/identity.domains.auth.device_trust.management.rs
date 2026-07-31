use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct DeviceTrustMutation {
    pub device_id: Uuid,
    pub trust_level: String,
    pub trust_score: i16,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn trust_device(
    db: &PgPool,
    principal_id: Uuid,
    device_id: Uuid,
) -> Result<DeviceTrustMutation, AppError> {
    let row = sqlx::query(
        r#"
        UPDATE account_devices
        SET trust_level = 'trusted',
            trust_score = GREATEST(trust_score, 80),
            trusted_at = COALESCE(trusted_at, NOW()),
            updated_at = NOW()
        WHERE id = $1 AND principal_id = $2 AND revoked_at IS NULL
        RETURNING id, trust_level, trust_score, revoked_at
        "#,
    )
    .bind(device_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("device_not_found", "Device not found."))?;

    Ok(mutation_from_row(row))
}

pub async fn revoke_device(
    db: &PgPool,
    principal_id: Uuid,
    device_id: Uuid,
) -> Result<DeviceTrustMutation, AppError> {
    let row = sqlx::query(
        r#"
        UPDATE account_devices
        SET trust_level = 'revoked',
            trust_score = 0,
            revoked_at = COALESCE(revoked_at, NOW()),
            updated_at = NOW()
        WHERE id = $1 AND principal_id = $2
        RETURNING id, trust_level, trust_score, revoked_at
        "#,
    )
    .bind(device_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("device_not_found", "Device not found."))?;

    Ok(mutation_from_row(row))
}

pub async fn revoke_all_devices(db: &PgPool, principal_id: Uuid) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE account_devices
        SET trust_level = 'revoked',
            trust_score = 0,
            revoked_at = COALESCE(revoked_at, NOW()),
            updated_at = NOW()
        WHERE principal_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

pub async fn revoke_all_devices_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE account_devices
        SET trust_level = 'revoked',
            trust_score = 0,
            revoked_at = COALESCE(revoked_at, NOW()),
            updated_at = NOW()
        WHERE principal_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected())
}

fn mutation_from_row(row: sqlx::postgres::PgRow) -> DeviceTrustMutation {
    DeviceTrustMutation {
        device_id: row.get("id"),
        trust_level: row.get("trust_level"),
        trust_score: row.get("trust_score"),
        revoked_at: row.get("revoked_at"),
    }
}
