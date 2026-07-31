use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{AccountError, AccountResult};

/// Account profile data used to render a privacy export notification.
#[derive(Debug)]
pub struct ProfileRecord {
    pub display_name: String,
}

pub async fn fetch_profile_record(db: &PgPool, principal_id: Uuid) -> AccountResult<ProfileRecord> {
    let row = sqlx::query(
        r#"
        SELECT
          u.firstname,
          u.lastname,
          u.username
        FROM users u
        WHERE u.principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AccountError::from)?;

    let firstname: Option<String> = row.get("firstname");
    let lastname: Option<String> = row.get("lastname");
    let username: Option<String> = row.get("username");
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );

    Ok(ProfileRecord { display_name })
}

pub fn derive_display_name(
    firstname: Option<&str>,
    lastname: Option<&str>,
    username: Option<&str>,
) -> String {
    let display_name = [firstname, lastname]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if !display_name.is_empty() {
        return display_name;
    }

    username
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("User")
        .to_string()
}
