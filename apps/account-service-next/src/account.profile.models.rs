use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileInput {
    pub firstname: NullableProfileText,
    pub lastname: NullableProfileText,
    pub username: NullableProfileText,
    pub birthdate: NullableProfileText,
    pub region: NullableProfileText,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct NullableProfileText(pub Option<String>);

#[derive(Debug)]
pub(crate) struct ValidatedProfileUpdate {
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AccountProfile {
    pub id: Uuid,
    pub display_name: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AccountProfileEnvelope {
    pub user: AccountProfile,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct AccountProfileRow {
    pub principal_id: Uuid,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_version: i64,
}

impl From<AccountProfileRow> for AccountProfile {
    fn from(row: AccountProfileRow) -> Self {
        let display_name = derive_display_name(
            row.firstname.as_deref(),
            row.lastname.as_deref(),
            row.username.as_deref(),
        );
        Self {
            id: row.principal_id,
            display_name,
            firstname: row.firstname,
            lastname: row.lastname,
            username: row.username,
            birthdate: row.birthdate,
            region: row.region,
            created_at: row.created_at,
        }
    }
}

pub(crate) fn derive_display_name(
    firstname: Option<&str>,
    lastname: Option<&str>,
    username: Option<&str>,
) -> String {
    let full_name = [firstname, lastname]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !full_name.is_empty() {
        return full_name;
    }
    username
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("User")
        .to_string()
}
