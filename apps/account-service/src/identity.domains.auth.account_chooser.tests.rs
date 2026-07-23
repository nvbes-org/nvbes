use super::account_entry_from_auth_context;
use crate::domains::auth::types::{AuthContext, SessionView};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

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
            device_id: None,
            device_trust_level: None,
            device_trust_score: None,
            risk_score: None,
            risk_decision: None,
            current: true,
        },
    );

    assert_eq!(account.authuser, "1");
    assert!(account.session.current);
}
