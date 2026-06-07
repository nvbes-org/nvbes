use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::{mfa, types::*};
use crate::http::error::AppError;

#[derive(Debug)]
pub struct UserRecord {
    pub principal_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
    pub password_hash: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn fetch_user_record(db: &PgPool, principal_id: Uuid) -> Result<UserRecord, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          u.birthdate,
          u.region,
          u.password_hash,
          u.email_verified_at,
          u.created_at,
          u.status::text AS status
        FROM users u
        WHERE u.principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let firstname: Option<String> = row.get("firstname");
    let lastname: Option<String> = row.get("lastname");
    let username: Option<String> = row.get("username");
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );

    Ok(UserRecord {
        principal_id: row.get("principal_id"),
        email: row.get("email"),
        display_name,
        status: row.get("status"),
        firstname,
        lastname,
        username,
        birthdate: row.get("birthdate"),
        region: row.get("region"),
        password_hash: row.get("password_hash"),
        email_verified_at: row.get("email_verified_at"),
        created_at: row.get("created_at"),
    })
}

pub async fn fetch_user_view(db: &PgPool, principal_id: Uuid) -> Result<UserView, AppError> {
    let user = fetch_user_record(db, principal_id).await?;
    Ok(UserView {
        id: user.principal_id,
        email: user.email,
        display_name: user.display_name,
        firstname: user.firstname,
        lastname: user.lastname,
        username: user.username,
        birthdate: user.birthdate,
        region: user.region,
        email_verified: user.email_verified_at.is_some(),
        mfa_enabled: mfa::has_active_factor(db, principal_id).await?,
        created_at: user.created_at,
    })
}
