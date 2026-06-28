use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserView {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<String>,
    pub region: Option<String>,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePolicyView {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceView {
    pub id: String,
    pub name: String,
    pub workspace_type: String,
    pub data_region: String,
    pub role: String,
    pub trial_ends_at: Option<String>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub plan_code: Option<String>,
    #[serde(default)]
    pub policy: Option<WorkspacePolicyView>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionView {
    pub id: String,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_region: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub session_id: String,
    pub workspace_id: Option<String>,
    pub tenant_id: Option<String>,
    pub scope: String,
    pub acr: Option<String>,
    pub amr: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub firstname: String,
    pub lastname: String,
    pub username: String,
    pub birthdate: Option<String>,
    pub region: Option<String>,
    pub workspace_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResult {
    pub user: UserView,
    pub workspace: WorkspaceView,
    pub verification_resend_available_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifierResult {
    pub next_step: String,
    pub state_token: String,
    pub available_methods: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaChallengeInput {
    pub state_token: String,
    pub totp_code: Option<String>,
    pub recovery_code: Option<String>,
    pub webauthn_response: Option<serde_json::Value>,
    pub webauthn_challenge_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub user: UserView,
    pub session: SessionView,
    #[serde(default)]
    pub session_token: Option<String>,
    pub verification_resend_available_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoginPasswordResult {
    Success(Box<LoginResult>),
    MfaRequired(IdentifierResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResult {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResult {
    pub user: UserView,
    pub current_tenant_id: Option<String>,
    pub current_organization_id: Option<String>,
    pub current_workspace_id: Option<String>,
    pub current_workspace_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesResult {
    pub workspaces: Vec<WorkspaceView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaFactorView {
    pub id: String,
    pub factor_type: String,
    pub kind: Option<String>,
    pub status: String,
    pub label: Option<String>,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaFactorsResult {
    pub factors: Vec<MfaFactorView>,
    pub mfa_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetupResult {
    pub factor: MfaFactorView,
    pub secret_base32: String,
    pub provisioning_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpConfirmResult {
    pub factor: MfaFactorView,
    pub mfa_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebauthnRegisterStartResult {
    pub factor_id: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebauthnAuthStartResult {
    pub challenge_id: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCodesResult {
    pub codes: Vec<String>,
}
