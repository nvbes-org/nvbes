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
    sqlx::query("INSERT INTO account_profiles (principal_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(principal_id)
        .execute(db)
        .await?;

    let result = sqlx::query(
        r#"
        UPDATE account_profiles
        SET firstname = COALESCE($2, firstname),
            lastname = COALESCE($3, lastname),
            username = COALESCE($4, username),
            birthdate = COALESCE($5, birthdate),
            region = COALESCE($6, region),
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
    .execute(db)
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
    fetch(db, principal_id).await
}

async fn fetch(db: &PgPool, principal_id: Uuid) -> Result<AccountProfile, AppError> {
    let row = sqlx::query_as::<_, AccountProfileRow>(
        r#"
        SELECT principal_id, firstname, lastname, username, birthdate, region, created_at
        FROM account_profiles
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    Ok(row.into())
}
