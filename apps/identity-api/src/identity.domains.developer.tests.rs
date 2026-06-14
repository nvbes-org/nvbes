use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

#[test]
fn developer_admin_has_all_developer_permissions() {
    let permissions = permissions_for_role(DeveloperRole::DeveloperAdmin);

    assert!(permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(permissions.contains(&DeveloperPermission::MarketplaceReview));
    assert!(permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(permissions.contains(&DeveloperPermission::WebhooksReplay));
    assert!(permissions.contains(&DeveloperPermission::LogsRead));
    assert!(permissions.contains(&DeveloperPermission::RbacManage));
}

#[test]
fn developer_integration_tester_can_use_tools_without_mutating_apps() {
    let permissions = permissions_for_role(DeveloperRole::IntegrationTester);

    assert!(permissions.contains(&DeveloperPermission::AppsRead));
    assert!(permissions.contains(&DeveloperPermission::TokensInspect));
    assert!(permissions.contains(&DeveloperPermission::HealthChecksRun));
    assert!(permissions.contains(&DeveloperPermission::SandboxUse));
    assert!(!permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(!permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(!permissions.contains(&DeveloperPermission::WebhooksManage));
}

#[test]
fn developer_docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, &[DeveloperPermission::DocsRead]);
}

#[test]
fn developer_role_db_literals_round_trip() {
    let cases = [
        (DeveloperRole::DeveloperAdmin, "developer_admin"),
        (DeveloperRole::AppManager, "app_manager"),
        (DeveloperRole::WebhookManager, "webhook_manager"),
        (DeveloperRole::LogViewer, "log_viewer"),
        (DeveloperRole::IntegrationTester, "integration_tester"),
        (DeveloperRole::DocsViewer, "docs_viewer"),
    ];

    for (role, literal) in cases {
        assert_eq!(role.as_db_str(), literal);
        assert_eq!(DeveloperRole::try_from(literal), Ok(role));
    }
}

#[test]
fn developer_role_rejects_invalid_db_literal() {
    let err = DeveloperRole::try_from("owner").expect_err("invalid literal should be rejected");

    assert_eq!(err.literal(), "owner");
}
