use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, settings_models::AccountNotifications};

pub async fn fetch_or_initialize(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<AccountNotifications, AppError> {
    sqlx::query(
        "INSERT INTO account_notifications (principal_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(principal_id)
    .execute(db)
    .await?;
    fetch(db, principal_id).await
}

pub async fn update(
    db: &PgPool,
    principal_id: Uuid,
    input: AccountNotifications,
) -> Result<AccountNotifications, AppError> {
    sqlx::query(
        r#"
        INSERT INTO account_notifications
          (principal_id, email, push, in_app, marketing_email)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (principal_id) DO UPDATE
        SET email = EXCLUDED.email,
            push = EXCLUDED.push,
            in_app = EXCLUDED.in_app,
            marketing_email = EXCLUDED.marketing_email,
            updated_at = NOW()
        "#,
    )
    .bind(principal_id)
    .bind(input.email)
    .bind(input.push)
    .bind(input.in_app)
    .bind(input.marketing_email)
    .execute(db)
    .await?;
    fetch(db, principal_id).await
}

async fn fetch(db: &PgPool, principal_id: Uuid) -> Result<AccountNotifications, AppError> {
    sqlx::query_as::<_, AccountNotifications>(
        r#"
        SELECT email, push, in_app, marketing_email
        FROM account_notifications
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}
