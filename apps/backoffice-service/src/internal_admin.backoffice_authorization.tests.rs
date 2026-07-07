use axum::http::HeaderMap;
use uuid::Uuid;

use super::*;

#[test]
fn platform_admin_can_run_every_tracked_permission() {
    for permission in all_permissions() {
        assert!(role_allows(BackofficeRole::PlatformAdmin, permission));
    }
}

#[test]
fn viewer_cannot_mutate() {
    for permission in all_permissions() {
        assert!(!role_allows(BackofficeRole::Viewer, permission));
    }
}

#[test]
fn specialized_roles_are_limited_to_their_centers() {
    assert_allowed_only(
        BackofficeRole::FinanceAdmin,
        &[
            BackofficePermission::BillingMutate,
            BackofficePermission::BillingPlatformMutate,
            BackofficePermission::RevenueMutate,
        ],
    );
    assert_allowed_only(
        BackofficeRole::ProductAdmin,
        &[
            BackofficePermission::EntitlementsMutate,
            BackofficePermission::UsageMutate,
        ],
    );
    assert_allowed_only(
        BackofficeRole::DeveloperAdmin,
        &[BackofficePermission::DeveloperMutate],
    );
    assert_allowed_only(
        BackofficeRole::OperationsAdmin,
        &[
            BackofficePermission::CommunicationsMutate,
            BackofficePermission::OperationsMutate,
        ],
    );
    assert_allowed_only(
        BackofficeRole::ComplianceAdmin,
        &[
            BackofficePermission::ComplianceMutate,
            BackofficePermission::RegionMutate,
        ],
    );
    assert_allowed_only(
        BackofficeRole::SecurityAdmin,
        &[
            BackofficePermission::AccessMutate,
            BackofficePermission::GovernanceMutate,
            BackofficePermission::RiskMutate,
            BackofficePermission::SecurityMutate,
            BackofficePermission::UserLifecycle,
        ],
    );
}

#[test]
fn support_agent_cannot_execute_critical_mutations() {
    for permission in all_permissions() {
        assert!(!role_allows(BackofficeRole::SupportAgent, permission));
    }
}

#[test]
fn confirmation_must_match_exactly_after_trim() {
    assert!(require_confirmation("SUSPEND TENANT", "SUSPEND TENANT").is_ok());
    assert!(require_confirmation(" SUSPEND TENANT ", "SUSPEND TENANT").is_ok());
    assert!(require_confirmation("suspend tenant", "SUSPEND TENANT").is_err());
}

#[test]
fn strong_confirmation_includes_target_fingerprint() {
    let target_id =
        Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").expect("uuid should parse");
    assert_eq!(
        strong_confirmation_code("HOLD INVOICE", target_id),
        "HOLD INVOICE 018F2F61"
    );
    assert!(require_strong_confirmation("HOLD INVOICE", "HOLD INVOICE", target_id).is_err());
    assert!(
        require_strong_confirmation("HOLD INVOICE 018F2F61", "HOLD INVOICE", target_id).is_ok()
    );
}

#[test]
fn idempotency_key_is_required_and_validated() {
    let headers = HeaderMap::new();
    assert!(require_idempotency_key(&headers).is_err());

    let mut headers = HeaderMap::new();
    headers.insert(IDEMPOTENCY_KEY_HEADER, " key-123 ".parse().unwrap());
    assert_eq!(require_idempotency_key(&headers).unwrap(), "key-123");
}

#[test]
fn operator_grant_verification_requires_actor_and_role_headers() {
    let actor_id = Uuid::new_v4();
    let mut headers = HeaderMap::new();
    headers.insert(ROLE_HEADER, "platform_admin".parse().unwrap());
    assert!(backoffice_actor(&headers).is_err());

    headers.insert(ACTOR_HEADER, actor_id.to_string().parse().unwrap());
    assert_eq!(backoffice_actor(&headers).unwrap(), actor_id);
    assert_eq!(
        backoffice_role(&headers).unwrap().metric_name(),
        "platform_admin"
    );
}

fn assert_allowed_only(role: BackofficeRole, allowed: &[BackofficePermission]) {
    for permission in all_permissions() {
        assert_eq!(
            role_allows(role, permission),
            allowed.contains(&permission),
            "{role:?} / {permission:?}"
        );
    }
}

fn all_permissions() -> [BackofficePermission; 17] {
    [
        BackofficePermission::AccessMutate,
        BackofficePermission::BillingMutate,
        BackofficePermission::BillingPlatformMutate,
        BackofficePermission::CommunicationsMutate,
        BackofficePermission::ComplianceMutate,
        BackofficePermission::DeveloperMutate,
        BackofficePermission::EntitlementsMutate,
        BackofficePermission::GovernanceMutate,
        BackofficePermission::OperationsMutate,
        BackofficePermission::RegionMutate,
        BackofficePermission::RevenueMutate,
        BackofficePermission::RiskMutate,
        BackofficePermission::SecurityMutate,
        BackofficePermission::TenantLifecycle,
        BackofficePermission::UsageMutate,
        BackofficePermission::UserLifecycle,
        BackofficePermission::WorkspaceLifecycle,
    ]
}
