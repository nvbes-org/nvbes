use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::{PublicKeyCredential, RegisterPublicKeyCredential};

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub firstname: String,
    pub lastname: String,
    pub username: String,
    pub birthdate: Option<chrono::NaiveDate>,
    pub region: Option<String>,
    pub data_region: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub legal_documents_accepted: bool,
    pub marketing_emails_accepted: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailInput {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordInput {
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordInput {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StepUpInput {
    pub password: Option<String>,
    pub totp_code: Option<String>,
    #[schema(value_type = Object)]
    pub webauthn_response: Option<PublicKeyCredential>,
    pub webauthn_challenge_id: Option<Uuid>,
    pub recovery_code: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SwitchWorkspaceInput {
    pub password: Option<String>,
    pub totp_code: Option<String>,
    #[schema(value_type = Object)]
    pub webauthn_response: Option<PublicKeyCredential>,
    pub webauthn_challenge_id: Option<Uuid>,
    pub recovery_code: Option<String>,
}

impl SwitchWorkspaceInput {
    pub fn into_step_up_input(self) -> StepUpInput {
        StepUpInput {
            password: self.password,
            totp_code: self.totp_code,
            webauthn_response: self.webauthn_response,
            webauthn_challenge_id: self.webauthn_challenge_id,
            recovery_code: self.recovery_code,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TotpSetupInput {
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TotpConfirmInput {
    pub factor_id: Uuid,
    pub code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WebauthnRegisterStartInput {
    pub label: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WebauthnRegisterFinishInput {
    pub factor_id: Uuid,
    #[schema(value_type = Object)]
    pub reg: RegisterPublicKeyCredential,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecoveryCodesGenerateInput {
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileInput {
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<chrono::NaiveDate>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub user_email: String,
    pub display_name: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub mfa_enabled: bool,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub session_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub scope: String,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub auth_time: Option<DateTime<Utc>>,
    pub client_id: Option<String>,
    pub cnf_jkt: Option<String>,
}

pub trait StepUpSubject {
    fn user_id(&self) -> Uuid;
    fn session_id(&self) -> Uuid;
    fn tenant_id(&self) -> Option<Uuid>;
    fn workspace_id(&self) -> Option<Uuid>;
}

impl StepUpSubject for AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn session_id(&self) -> Uuid {
        self.session_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }
}

impl From<&crate::http::middleware::jwt::AuthContext> for AuthContext {
    fn from(value: &crate::http::middleware::jwt::AuthContext) -> Self {
        Self {
            user_id: value.user_id,
            user_email: value.user_email.clone(),
            display_name: value.display_name.clone(),
            email_verified_at: value.email_verified_at,
            mfa_enabled: value.mfa_enabled,
            tenant_id: value.tenant_id,
            organization_id: value.organization_id,
            session_id: value.session_id,
            workspace_id: value.workspace_id,
            workspace_region: value.workspace_region.clone(),
            scope: value.scope.clone(),
            acr: value.acr.clone(),
            amr: value.amr.clone(),
            auth_time: value
                .auth_time
                .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0)),
            client_id: value.client_id.clone(),
            cnf_jkt: value.cnf_jkt.clone(),
        }
    }
}
