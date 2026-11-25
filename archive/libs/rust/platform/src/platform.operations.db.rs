use crate::operations_error::OperationsError;
use crate::operations_model::Case;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn load_case(db: &PgPool, id: Uuid) -> Result<Case, OperationsError> {
    let value: Value = sqlx::query_scalar("SELECT document FROM operations_cases WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(OperationsError::NotFound)?;
    Ok(serde_json::from_value(value)?)
}

pub async fn lock_case(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    version: i64,
) -> Result<Case, OperationsError> {
    let value: Value =
        sqlx::query_scalar("SELECT document FROM operations_cases WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(OperationsError::NotFound)?;
    let case: Case = serde_json::from_value(value)?;
    if case.version != version {
        return Err(OperationsError::Conflict);
    }
    Ok(case)
}

pub async fn save_case(
    tx: &mut Transaction<'_, Postgres>,
    case: &Case,
) -> Result<(), OperationsError> {
    sqlx::query("INSERT INTO operations_cases(id, version, document, updated_at) VALUES ($1,$2,$3,$4) ON CONFLICT(id) DO UPDATE SET version=$2, document=$3, updated_at=$4")
        .bind(case.id).bind(case.version).bind(serde_json::to_value(case)?).bind(case.updated_at)
        .execute(&mut **tx).await?;
    Ok(())
}
