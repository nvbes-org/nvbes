use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::record::fetch_user_record;
use crate::domains::auth::{mfa, types::*};
use crate::http::error::AppError;

pub async fn update_user_profile(
    db: &PgPool,
    principal_id: Uuid,
    input: &UpdateProfileInput,
) -> Result<UserView, AppError> {
    let existing = fetch_user_record(db, principal_id).await?;

    let firstname = input.firstname.clone().or(existing.firstname);
    let lastname = input.lastname.clone().or(existing.lastname);
    let username = input
        .username
        .as_deref()
        .map(crate::domains::auth::username::normalize_username)
        .transpose()?
        .or(existing.username);
    let birthdate = input.birthdate.or(existing.birthdate);
    let region = input.region.clone().or(existing.region);
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );

    let result = sqlx::query(
        r#"
        UPDATE users
        SET firstname = $2, lastname = $3, username = $4, birthdate = $5, region = $6, updated_at = NOW()
        WHERE principal_id = $1
        RETURNING email, email_verified_at, created_at
        "#,
    )
    .bind(principal_id)
    .bind(&firstname)
    .bind(&lastname)
    .bind(&username)
    .bind(birthdate)
    .bind(&region)
    .fetch_one(db)
    .await
    .map_err(|error| {
        if let sqlx::Error::Database(ref db_err) = error {
            if db_err.constraint() == Some("idx_users_username") {
                AppError::conflict("username_taken", "This username is already taken.")
            } else {
                AppError::internal("database_error", "Failed to update profile.")
            }
        } else {
            error.into()
        }
    })?;

    Ok(UserView {
        id: existing.principal_id,
        email: result.get("email"),
        display_name,
        firstname,
        lastname,
        username,
        birthdate,
        region,
        email_verified: result
            .get::<Option<DateTime<Utc>>, _>("email_verified_at")
            .is_some(),
        mfa_enabled: mfa::has_active_factor(db, principal_id).await?,
        created_at: result.get("created_at"),
    })
}
