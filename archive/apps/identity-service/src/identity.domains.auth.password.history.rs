use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn insert_password_hash(
    db: &PgPool,
    principal_id: Uuid,
    password_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO password_history (principal_id, password_hash)
        VALUES ($1, $2)
        "#,
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn insert_password_hash_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    password_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO password_history (principal_id, password_hash)
        VALUES ($1, $2)
        "#,
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn is_password_reused(
    db: &PgPool,
    principal_id: Uuid,
    new_password: &str,
    history_size: usize,
    password_pepper: Option<&str>,
) -> Result<bool, AppError> {
    if history_size == 0 {
        return Ok(false);
    }
    let hashes: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT password_hash
        FROM password_history
        WHERE principal_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(principal_id)
    .bind(history_size as i64)
    .fetch_all(db)
    .await?;

    for stored_hash in hashes {
        if nvbes_core::auth::verify_password_with_pepper(
            new_password,
            &stored_hash,
            password_pepper.map(str::as_bytes),
        )
        .unwrap_or(false)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn prune_history(
    db: &PgPool,
    principal_id: Uuid,
    history_size: usize,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM password_history
        WHERE principal_id = $1
          AND id NOT IN (
              SELECT id
              FROM password_history
              WHERE principal_id = $1
              ORDER BY created_at DESC
              LIMIT $2
          )
        "#,
    )
    .bind(principal_id)
    .bind(history_size as i64)
    .execute(db)
    .await?;
    Ok(())
}
