use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[path = "identity.domains.auth.types.inputs.rs"]
mod inputs;

pub use inputs::{
    ApproveRecoveryInput, AuthContext, ChangePasswordInput, ForgotPasswordInput, LoginInput,
    RecoveryCodesGenerateInput, RegisterInput, ResetPasswordInput, StepUpInput, StepUpSubject,
    SwitchWorkspaceInput, TotpConfirmInput, TotpSetupInput, UpdateProfileInput, VerifyEmailInput,
    WebauthnRegisterFinishInput, WebauthnRegisterStartInput,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EmailAddressView {
    pub id: Uuid,
    pub email: String,
    pub is_primary: bool,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EmailAddressesResult {
    pub emails: Vec<EmailAddressView>,
    pub primary_min_age_hours: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AddSecondaryEmailInput {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AddSecondaryEmailResult {
    pub email: EmailAddressView,
    pub verification_resend_available_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ResendSecondaryEmailVerificationResult {
    pub email: EmailAddressView,
    pub verification_resend_available_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct PromoteSecondaryEmailResult {
    pub email: EmailAddressView,
    pub user: UserView,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteSecondaryEmailResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceView {
    pub id: Uuid,
    pub owner_principal_id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: String,
    pub role: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SessionView {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub current: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterResult {
    pub user: UserView,
    pub workspace: WorkspaceView,
    pub verification_resend_available_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResult {
    pub user: UserView,
    pub session: SessionView,
    #[serde(skip_serializing)]
    pub session_token: String,
    pub verification_resend_available_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogoutResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyEmailResult {
    pub success: bool,
    pub user: UserView,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResendVerificationResult {
    pub success: bool,
    pub email_verified: bool,
    pub verification_resend_available_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordResult {
    pub success: bool,
    pub requires_admin_approval: bool,
    pub available_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApproveRecoveryResult {
    pub success: bool,
    pub available_at: Option<DateTime<Utc>>,
    pub requires_second_approval: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StepUpResult {
    pub success: bool,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwitchWorkspaceResult {
    pub workspace: WorkspaceView,
    pub session: SessionView,
    pub stepped_up: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaFactorView {
    pub id: Uuid,
    pub factor_type: String,
    pub kind: Option<String>,
    pub status: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaFactorsResult {
    pub factors: Vec<MfaFactorView>,
    pub mfa_enabled: bool,
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionsResult {
    pub sessions: Vec<SessionView>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MeResult {
    pub user: UserView,
    pub current_tenant_id: Option<Uuid>,
    pub current_organization_id: Option<Uuid>,
    pub current_workspace_id: Option<Uuid>,
    pub current_workspace_region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DataExportResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteAccountResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateProfileResult {
    pub user: UserView,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPreferences {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub skip_password: bool,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserNotifications {
    #[serde(default = "default_true")]
    pub email: bool,
    #[serde(default = "default_true")]
    pub push: bool,
    #[serde(default = "default_true")]
    pub in_app: bool,
    #[serde(default)]
    pub marketing_email: bool,
}

fn default_true() -> bool {
    true
}

pub fn derive_display_name(
    firstname: Option<&str>,
    lastname: Option<&str>,
    username: Option<&str>,
) -> String {
    let firstname = firstname.unwrap_or("").trim();
    let lastname = lastname.unwrap_or("").trim();

    if !firstname.is_empty() && !lastname.is_empty() {
        format!("{firstname} {lastname}")
    } else if let Some(username) = username.map(str::trim).filter(|value| !value.is_empty()) {
        username.to_string()
    } else {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::derive_display_name;

    #[test]
    fn derive_display_name_uses_firstname_and_lastname_when_both_present() {
        assert_eq!(
            derive_display_name(Some("Rayane"), Some("Guemmoud"), Some("shaynlink")),
            "Rayane Guemmoud"
        );
    }

    #[test]
    fn derive_display_name_falls_back_to_username_when_profile_name_is_incomplete() {
        assert_eq!(
            derive_display_name(Some("Rayane"), None, Some("shaynlink")),
            "shaynlink"
        );
    }
}
