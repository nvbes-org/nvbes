use super::{
    AccountChooserSessionStatus, account_entry_from_auth_context, account_entry_from_expired_claims,
};
use crate::domains::auth::types::{AuthContext, SessionView};
use crate::domains::auth::{db::UserRecord, jwt::types::TokenClaims};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

#[test]
fn expired_claims_stay_visible_for_reauthentication() {
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let created_at = Utc.with_ymd_and_hms(2026, 6, 9, 10, 0, 0).unwrap();
    let expires_at = Utc.with_ymd_and_hms(2026, 6, 9, 11, 0, 0).unwrap();
    let user = UserRecord {
        principal_id,
        email: "expired@example.test".to_string(),
        display_name: "Expired User".to_string(),
        status: "active".to_string(),
        firstname: Some("Expired".to_string()),
        lastname: Some("User".to_string()),
        username: Some("expired".to_string()),
        birthdate: None,
        region: Some("FR".to_string()),
        password_hash: None,
        email_verified_at: Some(created_at),
        created_at,
    };
    let claims = TokenClaims {
        jti: Uuid::new_v4().to_string(),
        sid: session_id.to_string(),
        sub: principal_id.to_string(),
        workspace_id: Some(Uuid::new_v4().to_string()),
        workspace_region: Some("eu".to_string()),
        tenant_id: Some(Uuid::new_v4().to_string()),
        organization_id: None,
        token_type: "access".to_string(),
        scope: "openid profile email".to_string(),
        authorization_details: Vec::new(),
        acr: Some("aal1".to_string()),
        amr: vec!["pwd".to_string()],
        client_id: Some("cloud-web".to_string()),
        auth_time: Some(created_at.timestamp()),
        iss: "nvbes-identity".to_string(),
        aud: "nvbes-account-service".to_string(),
        exp: expires_at.timestamp(),
        iat: created_at.timestamp(),
        nbf: created_at.timestamp(),
        cnf: None,
        act: None,
    };

    let account = account_entry_from_expired_claims("1", &claims, user, false)
        .expect("expired claim should produce an account chooser entry");

    assert_eq!(account.authuser, "1");
    assert_eq!(account.status, AccountChooserSessionStatus::Expired);
    assert_eq!(
        account.message.as_deref(),
        Some("Session expirée, veuillez vous reconnecter.")
    );
    assert_eq!(account.user.email, "expired@example.test");
    assert_eq!(account.session.id, session_id);
    assert_eq!(account.session.expires_at, expires_at);
}

#[test]
fn active_account_preserves_current_session_flag() {
    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap();
    let account = account_entry_from_auth_context(
        "1".to_string(),
        AuthContext {
            user_id,
            user_email: "current@example.test".to_string(),
            display_name: "Current User".to_string(),
            email_verified_at: Some(now),
            mfa_enabled: false,
            tenant_id: None,
            organization_id: None,
            workspace_id: None,
            workspace_region: None,
            session_id,
            scope: "openid profile email".to_string(),
            acr: Some("aal1".to_string()),
            amr: vec!["pwd".to_string()],
            auth_time: Some(now),
            client_id: None,
            cnf_jkt: None,
        },
        SessionView {
            id: session_id,
            tenant_id: None,
            organization_id: None,
            workspace_id: None,
            workspace_region: None,
            created_at: now,
            last_seen_at: now,
            expires_at: now,
            revoked_at: None,
            ip: None,
            user_agent: None,
            current: true,
        },
    );

    assert_eq!(account.authuser, "1");
    assert!(account.session.current);
}
