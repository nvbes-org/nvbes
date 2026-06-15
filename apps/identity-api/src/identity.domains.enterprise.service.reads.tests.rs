use super::{mfa_policy_view, session_policy_view};
use crate::domains::enterprise::db::{MfaPolicyRow, SessionPolicyRow};

#[test]
fn session_policy_view_uses_environment_source_when_tenant_policy_is_missing() {
    let view = session_policy_view(None, 12);

    assert_eq!(view.admin_session_ttl_hours, 12);
    assert_eq!(view.source, "environment");
    assert!(!view.compliant);
    assert_eq!(view.recommended_admin_session_ttl_hours, 8);
    assert!(view.step_up_required_for_admin_elevation);
}

#[test]
fn session_policy_view_marks_short_tenant_policy_compliant() {
    let view = session_policy_view(
        Some(SessionPolicyRow {
            admin_session_ttl_hours: Some(4),
        }),
        12,
    );

    assert_eq!(view.admin_session_ttl_hours, 4);
    assert_eq!(view.source, "tenant_policy");
    assert!(view.compliant);
}

#[test]
fn mfa_policy_view_requires_admin_or_all_enforcement_for_compliance() {
    let optional = mfa_policy_view(MfaPolicyRow {
        policy: "optional".to_string(),
    });
    let admin = mfa_policy_view(MfaPolicyRow {
        policy: "required_admins".to_string(),
    });
    let all = mfa_policy_view(MfaPolicyRow {
        policy: "required_all".to_string(),
    });

    assert!(!optional.compliant);
    assert!(admin.compliant);
    assert!(all.compliant);
    assert_eq!(admin.recommended_policy, "required_admins");
}
