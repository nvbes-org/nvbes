use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    profile_models::{AccountProfile, AccountProfileRow, ValidatedProfileUpdate},
};

pub async fn fetch_or_initialize(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<AccountProfile, AppError> {
    sqlx::query("INSERT INTO account_profiles (principal_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(principal_id)
        .execute(db)
        .await?;
    fetch(db, principal_id).await
}

pub async fn update(
    db: &PgPool,
    principal_id: Uuid,
    input: ValidatedProfileUpdate,
) -> Result<AccountProfile, AppError> {
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO account_profiles (principal_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query(
        r#"
        UPDATE account_profiles
        SET firstname = $2,
            lastname = $3,
            username = $4,
            birthdate = $5,
            region = $6,
            profile_version = profile_version + 1,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(input.firstname)
    .bind(input.lastname)
    .bind(input.username)
    .bind(input.birthdate)
    .bind(input.region)
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(error)) = &result {
        if error.constraint() == Some("account_profiles_username_unique") {
            return Err(AppError::conflict(
                "username_unavailable",
                "This Account username is already in use.",
            ));
        }
    }
    result?;
    let row = fetch_tx(&mut tx, principal_id).await?;
    crate::profile_projection::enqueue_tx(&mut tx, &row).await?;
    tx.commit().await?;
    Ok(row.into())
}

async fn fetch(db: &PgPool, principal_id: Uuid) -> Result<AccountProfile, AppError> {
    let row = sqlx::query_as::<_, AccountProfileRow>(
        r#"
        SELECT principal_id, firstname, lastname, username, birthdate, region,
               created_at, updated_at, profile_version
        FROM account_profiles
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    Ok(row.into())
}

async fn fetch_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
) -> Result<AccountProfileRow, AppError> {
    sqlx::query_as::<_, AccountProfileRow>(
        r#"
        SELECT principal_id, firstname, lastname, username, birthdate, region,
               created_at, updated_at, profile_version
        FROM account_profiles
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(Into::into)
}
