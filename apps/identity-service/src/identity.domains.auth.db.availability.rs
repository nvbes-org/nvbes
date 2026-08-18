use sqlx::PgPool;

use crate::http::error::AppError;

pub async fn registration_email_exists(
    db: &PgPool,
    normalized_email: &str,
) -> Result<bool, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_email_addresses
            WHERE normalized_email = $1
              AND deleted_at IS NULL
        )
        "#,
    )
    .bind(normalized_email)
    .fetch_one(db)
    .await
    .map_err(Into::into)
}
