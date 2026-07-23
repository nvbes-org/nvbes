use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::oauth::rar::AuthorizationDetails;

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizationCodeView {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenView {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    pub scope: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    #[schema(value_type = Vec<Object>)]
    pub authorization_details: AuthorizationDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_token_type: Option<String>,
}
