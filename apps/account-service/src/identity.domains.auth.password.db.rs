use crate::http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.auth.password.db.reset.rs"]
pub mod reset;

pub async fn find_principal_and_display_name_by_email(
    db: &PgPool,
    email: &str,
) -> Result<Option<(Uuid, String)>, AppError> {
    let row = sqlx::query(
        r#"SELECT principal_id, firstname, lastname, username FROM users WHERE lower(email) = lower($1) LIMIT 1"#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?;

    if let Some(row) = row {
        use sqlx::Row;
        let firstname: Option<String> = row.get("firstname");
        let lastname: Option<String> = row.get("lastname");
        let username: Option<String> = row.get("username");
        Ok(Some((
            row.get("principal_id"),
            crate::domains::auth::types::derive_display_name(
                firstname.as_deref(),
                lastname.as_deref(),
                username.as_deref(),
            ),
        )))
    } else {
        Ok(None)
    }
}
