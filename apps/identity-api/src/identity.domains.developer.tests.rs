use super::rbac::{DeveloperPermission, DeveloperRole, has_permission, permissions_for_role};
use super::rbac_db::{parse_developer_role, permissions_for_roles};
use super::types::DeveloperWebhookEventType;

#[test]
fn developer_admin_has_all_developer_permissions() {
    let permissions = permissions_for_role(DeveloperRole::DeveloperAdmin);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::LogsRead,
            DeveloperPermission::TokensInspect,
            DeveloperPermission::OAuthPlayground,
            DeveloperPermission::RbacManage,
            DeveloperPermission::DocsRead,
        ]
    );
}

#[test]
fn app_manager_can_manage_apps_and_read_docs() {
    let permissions = permissions_for_role(DeveloperRole::AppManager);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::DocsRead,
        ]
    );
}

#[test]
fn webhook_manager_can_manage_webhooks_and_read_apps_and_docs() {
    let permissions = permissions_for_role(DeveloperRole::WebhookManager);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ]
    );
}

#[test]
fn log_viewer_can_read_logs_apps_webhooks_and_docs() {
    let permissions = permissions_for_role(DeveloperRole::LogViewer);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::LogsRead,
            DeveloperPermission::AppsRead,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::DocsRead,
        ]
    );
}

#[test]
fn integration_tester_can_use_tools_without_mutating_apps() {
    let permissions = permissions_for_role(DeveloperRole::IntegrationTester);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::TokensInspect,
            DeveloperPermission::OAuthPlayground,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ]
    );
}

#[test]
fn docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, vec![DeveloperPermission::DocsRead]);
}

#[test]
fn developer_role_serializes_as_snake_case() {
    let serialized = serde_json::to_string(&DeveloperRole::DeveloperAdmin).unwrap();

    assert_eq!(serialized, "\"developer_admin\"");
}

#[test]
fn developer_permission_serializes_as_dotted_scope() {
    let serialized = serde_json::to_string(&DeveloperPermission::AppsRead).unwrap();

    assert_eq!(serialized, "\"developer.apps.read\"");
}

#[test]
fn developer_webhook_event_serializes_as_dotted_event_type() {
    let serialized = serde_json::to_string(&DeveloperWebhookEventType::UserCreated).unwrap();

    assert_eq!(serialized, "\"user.created\"");
}

#[test]
fn developer_webhook_event_exposes_database_event_type() {
    assert_eq!(
        DeveloperWebhookEventType::SessionRevoked.as_event_type(),
        "session.revoked"
    );
}

#[test]
fn developer_permission_scope_matches_serialized_scope() {
    assert_eq!(
        DeveloperPermission::AppsUpdateRedirects.as_scope(),
        "developer.apps.update_redirects"
    );
}

#[test]
fn has_permission_matches_exact_permission() {
    let permissions = [
        DeveloperPermission::AppsRead,
        DeveloperPermission::AppsUpdateRedirects,
    ];

    assert!(has_permission(
        &permissions,
        DeveloperPermission::AppsUpdateRedirects
    ));
    assert!(!has_permission(
        &permissions,
        DeveloperPermission::AppsRevoke
    ));
}

#[test]
fn parses_all_developer_role_database_values() {
    let roles = [
        DeveloperRole::DeveloperAdmin,
        DeveloperRole::AppManager,
        DeveloperRole::WebhookManager,
        DeveloperRole::LogViewer,
        DeveloperRole::IntegrationTester,
        DeveloperRole::DocsViewer,
    ];

    for role in roles {
        assert_eq!(parse_developer_role(role.as_db_str()).unwrap(), role);
    }
}

#[test]
fn permissions_for_roles_dedupes_overlapping_permissions() {
    let permissions = permissions_for_roles(&[
        DeveloperRole::AppManager,
        DeveloperRole::WebhookManager,
        DeveloperRole::DocsViewer,
    ]);

    assert_eq!(
        permissions,
        vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::DocsRead,
        ]
    );
}
