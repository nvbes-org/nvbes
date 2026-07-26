use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Service-account projection exposed by Developer Console.
#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperServiceAccountSummary {
    pub principal_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role: String,
    pub status: String,
    pub workspace_id: Uuid,
    pub oauth_client_count: i64,
    pub last_rotated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperServiceAccountsResponse {
    pub service_accounts: Vec<DeveloperServiceAccountSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperSecretVersionSummary {
    pub id: Uuid,
    pub client_id: String,
    pub status: String,
    pub secret_last4: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperSecretVersionsResponse {
    pub secret_versions: Vec<DeveloperSecretVersionSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RotateDeveloperSecretInput {
    pub overlap_hours: i64,
}

#[derive(Debug, Serialize)]
pub struct RotateDeveloperSecretResponse {
    pub client_id: String,
    pub client_secret: String,
    pub active_version_id: Uuid,
    pub previous_version_id: Uuid,
    pub overlap_ends_at: DateTime<Utc>,
    pub rotated_at: DateTime<Utc>,
}
