use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaFactorView {
    pub id: Uuid,
    pub factor_type: String,
    pub kind: Option<String>,
    pub assurance: Option<String>,
    pub phishing_resistant: bool,
    pub backup_eligible: Option<bool>,
    pub backup_state: Option<bool>,
    pub sign_count: Option<i64>,
    pub attestation_format: Option<String>,
    pub status: String,
    pub label: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaFactorsResult {
    pub factors: Vec<MfaFactorView>,
    pub mfa_enabled: bool,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TotpSetupResult {
    pub factor: MfaFactorView,
    pub secret_base32: String,
    pub provisioning_uri: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TotpConfirmResult {
    pub factor: MfaFactorView,
    pub mfa_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebauthnAuthStartResult {
    pub challenge_id: Uuid,
    pub options: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebauthnRegisterStartResult {
    pub factor_id: Uuid,
    pub options: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryCodesResult {
    pub codes: Vec<String>,
}
