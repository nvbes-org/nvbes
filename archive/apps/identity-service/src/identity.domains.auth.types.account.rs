use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
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
    pub next_cursor: Option<String>,
    pub has_more: bool,
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
pub struct RegisterResult {
    pub user: UserView,
    pub verification_resend_available_at: DateTime<Utc>,
    #[serde(skip_serializing, skip_deserializing)]
    pub registration_enrollment_token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResult {
    pub user: UserView,
    pub session: super::SessionView,
    #[serde(skip_serializing)]
    pub browser_session_token: String,
    #[serde(skip_serializing)]
    pub device_cookie_token: Option<String>,
    pub verification_resend_available_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub registration_enrollment_token: Option<String>,
}

macro_rules! success_result {
    ($name:ident) => {
        #[derive(Debug, Serialize, Deserialize, ToSchema)]
        pub struct $name {
            pub success: bool,
        }
    };
}

success_result!(LogoutResult);
success_result!(ForgotPasswordResult);
success_result!(ResetPasswordResult);
success_result!(ChangePasswordResult);

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
pub struct StepUpResult {
    pub success: bool,
    pub valid_until: DateTime<Utc>,
    #[serde(skip_serializing, skip_deserializing)]
    pub browser_session_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EmailStepUpChallengeResult {
    pub challenge_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

pub const DEFAULT_DISPLAY_NAME: &str = "User";
