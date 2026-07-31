use utoipa::openapi::security::SecurityRequirement;

use crate::http::middleware::jwt::account_access::{
    DELETE_SCOPE, EMAIL_READ_SCOPE, EMAIL_WRITE_SCOPE, EXPORT_SCOPE, LEGAL_READ_SCOPE,
    LEGAL_WRITE_SCOPE, OAUTH_APPROVAL_SCOPE, OAUTH_CLIENTS_READ_SCOPE, OAUTH_CLIENTS_WRITE_SCOPE,
    PREFERENCES_READ_SCOPE, PREFERENCES_WRITE_SCOPE, PROFILE_READ_SCOPE, PROFILE_WRITE_SCOPE,
    SECURITY_READ_SCOPE, SECURITY_WRITE_SCOPE, SESSION_READ_SCOPE, SESSION_WRITE_SCOPE,
};

use super::{
    BROWSER_SESSION_SCHEME, OAUTH_CLIENT_BASIC_SCHEME, OAUTH_CLIENT_MTLS_SCHEME, OAUTH2_SCHEME,
    REGISTRATION_ENROLLMENT_SCHEME,
};

#[derive(Clone, Copy)]
pub(super) enum OperationSecurity {
    Public,
    RegistrationEnrollment,
    BrowserSession,
    BrowserOrOAuth(&'static str),
    OAuthClient,
    OAuthResource,
}

impl OperationSecurity {
    pub(super) fn requirements(self) -> Vec<SecurityRequirement> {
        match self {
            Self::Public => Vec::new(),
            Self::RegistrationEnrollment => vec![SecurityRequirement::new(
                REGISTRATION_ENROLLMENT_SCHEME,
                Vec::<&str>::new(),
            )],
            Self::BrowserSession => vec![SecurityRequirement::new(
                BROWSER_SESSION_SCHEME,
                Vec::<&str>::new(),
            )],
            Self::BrowserOrOAuth(scope) => vec![
                SecurityRequirement::new(BROWSER_SESSION_SCHEME, Vec::<&str>::new()),
                SecurityRequirement::new(OAUTH2_SCHEME, [scope]),
            ],
            Self::OAuthClient => vec![
                SecurityRequirement::new(OAUTH_CLIENT_BASIC_SCHEME, Vec::<&str>::new()),
                SecurityRequirement::new(OAUTH_CLIENT_MTLS_SCHEME, Vec::<&str>::new()),
            ],
            Self::OAuthResource => {
                vec![SecurityRequirement::new(OAUTH2_SCHEME, Vec::<&str>::new())]
            }
        }
    }

    pub(super) fn admits_browser_session(self) -> bool {
        matches!(self, Self::BrowserSession | Self::BrowserOrOAuth(_))
    }

    pub(super) fn requires_csrf_header(self) -> bool {
        matches!(self, Self::BrowserSession)
    }
}

pub(super) fn classify(operation_id: &str) -> Option<OperationSecurity> {
    let security = match operation_id {
        "change_verify_email" => OperationSecurity::RegistrationEnrollment,
        "logout" | "step_up" => OperationSecurity::BrowserSession,
        "me" | "me_avatar" => OperationSecurity::BrowserOrOAuth(PROFILE_READ_SCOPE),
        "me_update" | "me_avatar_upload" | "me_avatar_delete" => {
            OperationSecurity::BrowserOrOAuth(PROFILE_WRITE_SCOPE)
        }
        "me_emails_get" => OperationSecurity::BrowserOrOAuth(EMAIL_READ_SCOPE),
        "me_emails_post"
        | "me_email_promote"
        | "me_email_resend_verification"
        | "me_email_delete" => OperationSecurity::BrowserOrOAuth(EMAIL_WRITE_SCOPE),
        "list_sessions" => OperationSecurity::BrowserOrOAuth(SESSION_READ_SCOPE),
        "revoke_session"
        | "revoke_all_sessions"
        | "revoke_all_other_sessions"
        | "confirm_high_risk_session" => OperationSecurity::BrowserOrOAuth(SESSION_WRITE_SCOPE),
        "list_mfa_factors" => OperationSecurity::BrowserOrOAuth(SECURITY_READ_SCOPE),
        "request_email_step_up"
        | "trust_device"
        | "revoke_device"
        | "begin_totp_enrollment"
        | "confirm_totp_enrollment"
        | "webauthn_auth_start"
        | "begin_webauthn_enrollment"
        | "confirm_webauthn_enrollment"
        | "generate_recovery_codes"
        | "remove_mfa_factor"
        | "change_password" => OperationSecurity::BrowserOrOAuth(SECURITY_WRITE_SCOPE),
        "me_preferences_get" | "me_notifications_get" => {
            OperationSecurity::BrowserOrOAuth(PREFERENCES_READ_SCOPE)
        }
        "me_preferences_put" | "me_notifications_put" => {
            OperationSecurity::BrowserOrOAuth(PREFERENCES_WRITE_SCOPE)
        }
        "me_export" | "me_export_download" => OperationSecurity::BrowserOrOAuth(EXPORT_SCOPE),
        "me_delete" => OperationSecurity::BrowserOrOAuth(DELETE_SCOPE),
        "list_consents" | "gpc_status" => OperationSecurity::BrowserOrOAuth(LEGAL_READ_SCOPE),
        "grant_consent" | "revoke_consent" => OperationSecurity::BrowserOrOAuth(LEGAL_WRITE_SCOPE),
        "list_clients" | "list_client_policies" | "list_client_keys" => {
            OperationSecurity::BrowserOrOAuth(OAUTH_CLIENTS_READ_SCOPE)
        }
        "create_client"
        | "revoke_client"
        | "create_client_policy"
        | "update_client_policy"
        | "delete_client_policy"
        | "rotate_client_key"
        | "revoke_client_key" => OperationSecurity::BrowserOrOAuth(OAUTH_CLIENTS_WRITE_SCOPE),
        "device_approve" | "device_deny" => OperationSecurity::BrowserOrOAuth(OAUTH_APPROVAL_SCOPE),
        "userinfo" | "decide" | "list_security_events" | "export_security_events" => {
            OperationSecurity::OAuthResource
        }
        "introspect" | "revoke" => OperationSecurity::OAuthClient,
        id if is_public_operation(id) => OperationSecurity::Public,
        _ => return None,
    };
    Some(security)
}

fn is_public_operation(operation_id: &str) -> bool {
    matches!(
        operation_id,
        "change_password_well_known"
            | "gpc_well_known"
            | "get_jwks"
            | "oauth_authorization_server_metadata"
            | "openid_configuration"
            | "passkey_endpoints_well_known"
            | "webauthn_well_known"
            | "get_accounts"
            | "challenge_identifier"
            | "challenge_mfa"
            | "challenge_pow"
            | "challenge_pwd"
            | "challenge_webauthn_start"
            | "challenge_webauthn_discoverable_start"
            | "challenge_webauthn_discoverable_finish"
            | "forgot_password"
            | "reset_password"
            | "region"
            | "supported_regions"
            | "register"
            | "registration_availability"
            | "verify_email"
            | "resend_verify_email"
            | "authorize"
            | "device_authorize"
            | "device_verify"
            | "token"
    )
}
