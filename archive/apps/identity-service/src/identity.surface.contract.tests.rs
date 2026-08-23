const USER_VIEW: &str = include_str!("identity.domains.auth.types.account.rs");
const SESSION_ROUTES: &str = include_str!("identity.domains.auth.routes.session_mgmt.router.rs");
const MFA_ROUTES: &str = include_str!("identity.domains.auth.routes.mfa.rs");
const MFA_COMMON_ROUTES: &str = include_str!("identity.domains.auth.routes.mfa.common.rs");
const MFA_TOTP_ROUTES: &str = include_str!("identity.domains.auth.routes.mfa.totp.rs");
const MFA_RECOVERY_ROUTES: &str = include_str!("identity.domains.auth.routes.mfa.recovery.rs");
const MFA_WEBAUTHN_ROUTES: &str = include_str!("identity.domains.auth.routes.mfa.webauthn.rs");
const PASSWORD_ROUTES: &str = include_str!("identity.domains.auth.routes.password.rs");
const OAUTH_CLIENT_ROUTES: &str = include_str!("identity.domains.oauth.routes.clients.rs");
const OAUTH_CLIENT_KEY_ROUTES: &str = include_str!("identity.domains.oauth.routes.clients.keys.rs");
const OIDC_PROJECTION: &str = include_str!("identity.domains.auth.oidc_profile_projection.rs");

#[test]
fn identity_user_contract_contains_no_account_profile_fields() {
    for forbidden in ["firstname", "lastname", "username", "birthdate", "region"] {
        assert!(
            !USER_VIEW.contains(&format!("pub {forbidden}:")),
            "Identity UserView still exposes Account field `{forbidden}`"
        );
    }
}

#[test]
fn account_management_primitives_require_oauth_tokens() {
    for source in [
        SESSION_ROUTES,
        MFA_ROUTES,
        MFA_COMMON_ROUTES,
        MFA_TOTP_ROUTES,
        MFA_RECOVERY_ROUTES,
        MFA_WEBAUTHN_ROUTES,
        PASSWORD_ROUTES,
        OAUTH_CLIENT_ROUTES,
        OAUTH_CLIENT_KEY_ROUTES,
    ] {
        assert!(!source.contains("BrowserSessionOrOAuthScope"));
    }
    assert!(!SESSION_ROUTES.contains("\"/me\""));
}

#[test]
fn oidc_projection_transports_only_oidc_profile_claims() {
    assert!(!OIDC_PROJECTION.contains("region"));
    assert!(OIDC_PROJECTION.contains("profile_version"));
}
