use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperRole {
    DeveloperAdmin,
    AppManager,
    WebhookManager,
    LogViewer,
    IntegrationTester,
    DocsViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DeveloperPermission {
    #[serde(rename = "developer.apps.read")]
    AppsRead,
    #[serde(rename = "developer.apps.create")]
    AppsCreate,
    #[serde(rename = "developer.apps.update_redirects")]
    AppsUpdateRedirects,
    #[serde(rename = "developer.apps.revoke")]
    AppsRevoke,
    #[serde(rename = "developer.webhooks.read")]
    WebhooksRead,
    #[serde(rename = "developer.webhooks.manage")]
    WebhooksManage,
    #[serde(rename = "developer.logs.read")]
    LogsRead,
    #[serde(rename = "developer.tokens.inspect")]
    TokensInspect,
    #[serde(rename = "developer.oauth.playground")]
    OAuthPlayground,
    #[serde(rename = "developer.rbac.manage")]
    RbacManage,
    #[serde(rename = "developer.docs.read")]
    DocsRead,
}

pub fn permissions_for_role(role: DeveloperRole) -> Vec<DeveloperPermission> {
    match role {
        DeveloperRole::DeveloperAdmin => vec![
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
        ],
        DeveloperRole::AppManager => vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::WebhookManager => vec![
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::LogViewer => vec![
            DeveloperPermission::LogsRead,
            DeveloperPermission::AppsRead,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::IntegrationTester => vec![
            DeveloperPermission::TokensInspect,
            DeveloperPermission::OAuthPlayground,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::DocsViewer => vec![DeveloperPermission::DocsRead],
    }
}

impl DeveloperRole {
    pub fn as_db_str(self) -> &'static str {
        match self {
            DeveloperRole::DeveloperAdmin => "developer_admin",
            DeveloperRole::AppManager => "app_manager",
            DeveloperRole::WebhookManager => "webhook_manager",
            DeveloperRole::LogViewer => "log_viewer",
            DeveloperRole::IntegrationTester => "integration_tester",
            DeveloperRole::DocsViewer => "docs_viewer",
        }
    }
}
