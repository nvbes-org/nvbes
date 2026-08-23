use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuthClientKeyPurpose {
    ClientAuthentication,
    RequestObject,
}

impl OAuthClientKeyPurpose {
    pub(super) fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "client_authentication" => Ok(Self::ClientAuthentication),
            "request_object" => Ok(Self::RequestObject),
            _ => Err(AppError::bad_request(
                "invalid_client_key_purpose",
                "purpose must be client_authentication or request_object.",
            )),
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::ClientAuthentication => "client_authentication",
            Self::RequestObject => "request_object",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RotateOAuthClientKeyInput {
    pub purpose: String,
    pub jwk: serde_json::Value,
    #[serde(default)]
    pub retire_previous_after_seconds: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthClientKeyView {
    pub id: Uuid,
    pub purpose: String,
    pub kid: String,
    pub jwk: serde_json::Value,
    pub status: String,
    pub activated_at: DateTime<Utc>,
    pub retire_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}
