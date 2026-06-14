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
fn integration_tester_can_use_tools_without_mutating_apps() {
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
fn docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, vec![DeveloperPermission::DocsRead]);
}
