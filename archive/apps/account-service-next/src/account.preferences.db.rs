use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, settings_models::AccountPreferences};

pub async fn fetch_or_initialize(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<AccountPreferences, AppError> {
    sqlx::query(
        "INSERT INTO account_preferences (principal_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(principal_id)
    .execute(db)
    .await?;
    fetch(db, principal_id).await
}

pub async fn update(
    db: &PgPool,
    principal_id: Uuid,
    input: AccountPreferences,
) -> Result<AccountPreferences, AppError> {
    sqlx::query(
        r#"
        INSERT INTO account_preferences (principal_id, theme, language)
        VALUES ($1, $2, $3)
        ON CONFLICT (principal_id) DO UPDATE
        SET theme = EXCLUDED.theme,
            language = EXCLUDED.language,
            updated_at = NOW()
        "#,
    )
    .bind(principal_id)
    .bind(input.theme)
    .bind(input.language)
    .execute(db)
    .await?;
    fetch(db, principal_id).await
}

async fn fetch(db: &PgPool, principal_id: Uuid) -> Result<AccountPreferences, AppError> {
    sqlx::query_as::<_, AccountPreferences>(
        "SELECT theme, language FROM account_preferences WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}
