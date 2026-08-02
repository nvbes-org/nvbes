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
        r#"
        SELECT u.principal_id, COALESCE(profile.display_name, 'User') AS display_name
        FROM users u
        LEFT JOIN identity_oidc_profile_claims profile
          ON profile.principal_id = u.principal_id
        WHERE lower(u.email) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?;

    if let Some(row) = row {
        use sqlx::Row;
        Ok(Some((row.get("principal_id"), row.get("display_name"))))
    } else {
        Ok(None)
    }
}
