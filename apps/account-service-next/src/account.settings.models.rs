use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum AccountTheme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum AccountLanguage {
    Fr,
    En,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountPreferences {
    pub theme: AccountTheme,
    pub language: AccountLanguage,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountNotifications {
    pub email: bool,
    pub push: bool,
    pub in_app: bool,
    pub marketing_email: bool,
}
