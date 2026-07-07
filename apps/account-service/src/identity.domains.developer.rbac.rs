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
    #[serde(rename = "docs.read")]
    ConsoleDocsRead,
    #[serde(rename = "apps.read")]
    ConsoleAppsRead,
    #[serde(rename = "apps.create")]
    ConsoleAppsCreate,
    #[serde(rename = "apps.update")]
    ConsoleAppsUpdate,
    #[serde(rename = "apps.revoke")]
    ConsoleAppsRevoke,
    #[serde(rename = "marketplace.read")]
    ConsoleMarketplaceRead,
    #[serde(rename = "marketplace.submit")]
    ConsoleMarketplaceSubmit,
    #[serde(rename = "marketplace.review")]
    MarketplaceReview,
    #[serde(rename = "scopes.read")]
    ConsoleScopesRead,
    #[serde(rename = "scopes.manage")]
    ConsoleScopesManage,
    #[serde(rename = "secrets.rotate")]
    SecretsRotate,
    #[serde(rename = "service_accounts.read")]
    ConsoleServiceAccountsRead,
    #[serde(rename = "service_accounts.manage")]
    ConsoleServiceAccountsManage,
    #[serde(rename = "webhooks.read")]
    ConsoleWebhooksRead,
    #[serde(rename = "webhooks.manage")]
    ConsoleWebhooksManage,
    #[serde(rename = "webhooks.replay")]
    WebhooksReplay,
    #[serde(rename = "logs.read")]
    ConsoleLogsRead,
    #[serde(rename = "tokens.inspect")]
    ConsoleTokensInspect,
    #[serde(rename = "health_checks.read")]
    ConsoleHealthChecksRead,
    #[serde(rename = "health_checks.run")]
    HealthChecksRun,
    #[serde(rename = "sandbox.use")]
    SandboxUse,
    #[serde(rename = "rbac.manage")]
    ConsoleRbacManage,
}

pub fn permissions_for_role(role: DeveloperRole) -> Vec<DeveloperPermission> {
    use DeveloperPermission::*;

    match role {
        DeveloperRole::DeveloperAdmin => vec![
            AppsRead,
            AppsCreate,
            AppsUpdateRedirects,
            AppsRevoke,
            WebhooksRead,
            WebhooksManage,
            LogsRead,
            TokensInspect,
            OAuthPlayground,
            RbacManage,
            DocsRead,
            ConsoleDocsRead,
            ConsoleAppsRead,
            ConsoleAppsCreate,
            ConsoleAppsUpdate,
            ConsoleAppsRevoke,
            ConsoleMarketplaceRead,
            ConsoleMarketplaceSubmit,
            MarketplaceReview,
            ConsoleScopesRead,
            ConsoleScopesManage,
            SecretsRotate,
            ConsoleServiceAccountsRead,
            ConsoleServiceAccountsManage,
            ConsoleWebhooksRead,
            ConsoleWebhooksManage,
            WebhooksReplay,
            ConsoleLogsRead,
            ConsoleTokensInspect,
            ConsoleHealthChecksRead,
            HealthChecksRun,
            SandboxUse,
            ConsoleRbacManage,
        ],
        DeveloperRole::AppManager => vec![
            AppsRead,
            AppsCreate,
            AppsUpdateRedirects,
            AppsRevoke,
            DocsRead,
            ConsoleDocsRead,
            ConsoleAppsRead,
            ConsoleAppsCreate,
            ConsoleAppsUpdate,
            ConsoleAppsRevoke,
            ConsoleMarketplaceRead,
            ConsoleMarketplaceSubmit,
            ConsoleScopesRead,
            SecretsRotate,
            ConsoleServiceAccountsRead,
            ConsoleServiceAccountsManage,
            ConsoleHealthChecksRead,
        ],
        DeveloperRole::WebhookManager => vec![
            WebhooksRead,
            WebhooksManage,
            AppsRead,
            DocsRead,
            ConsoleDocsRead,
            ConsoleWebhooksRead,
            ConsoleWebhooksManage,
            WebhooksReplay,
            ConsoleLogsRead,
            ConsoleHealthChecksRead,
            HealthChecksRun,
        ],
        DeveloperRole::LogViewer => vec![
            LogsRead,
            AppsRead,
            WebhooksRead,
            DocsRead,
            ConsoleDocsRead,
            ConsoleAppsRead,
            ConsoleWebhooksRead,
            ConsoleLogsRead,
            ConsoleHealthChecksRead,
        ],
        DeveloperRole::IntegrationTester => vec![
            TokensInspect,
            OAuthPlayground,
            AppsRead,
            DocsRead,
            ConsoleDocsRead,
            ConsoleAppsRead,
            ConsoleMarketplaceRead,
            ConsoleScopesRead,
            ConsoleServiceAccountsRead,
            ConsoleWebhooksRead,
            ConsoleLogsRead,
            ConsoleTokensInspect,
            ConsoleHealthChecksRead,
            HealthChecksRun,
            SandboxUse,
        ],
        DeveloperRole::DocsViewer => vec![DocsRead],
    }
}

pub fn has_permission(permissions: &[DeveloperPermission], required: DeveloperPermission) -> bool {
    permissions.contains(&required)
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

impl DeveloperPermission {
    pub fn as_scope(self) -> &'static str {
        self.as_api_str()
    }

    pub fn as_api_str(self) -> &'static str {
        match self {
            DeveloperPermission::AppsRead => "developer.apps.read",
            DeveloperPermission::AppsCreate => "developer.apps.create",
            DeveloperPermission::AppsUpdateRedirects => "developer.apps.update_redirects",
            DeveloperPermission::AppsRevoke => "developer.apps.revoke",
            DeveloperPermission::WebhooksRead => "developer.webhooks.read",
            DeveloperPermission::WebhooksManage => "developer.webhooks.manage",
            DeveloperPermission::LogsRead => "developer.logs.read",
            DeveloperPermission::TokensInspect => "developer.tokens.inspect",
            DeveloperPermission::OAuthPlayground => "developer.oauth.playground",
            DeveloperPermission::RbacManage => "developer.rbac.manage",
            DeveloperPermission::DocsRead => "developer.docs.read",
            DeveloperPermission::ConsoleDocsRead => "docs.read",
            DeveloperPermission::ConsoleAppsRead => "apps.read",
            DeveloperPermission::ConsoleAppsCreate => "apps.create",
            DeveloperPermission::ConsoleAppsUpdate => "apps.update",
            DeveloperPermission::ConsoleAppsRevoke => "apps.revoke",
            DeveloperPermission::ConsoleMarketplaceRead => "marketplace.read",
            DeveloperPermission::ConsoleMarketplaceSubmit => "marketplace.submit",
            DeveloperPermission::MarketplaceReview => "marketplace.review",
            DeveloperPermission::ConsoleScopesRead => "scopes.read",
            DeveloperPermission::ConsoleScopesManage => "scopes.manage",
            DeveloperPermission::SecretsRotate => "secrets.rotate",
            DeveloperPermission::ConsoleServiceAccountsRead => "service_accounts.read",
            DeveloperPermission::ConsoleServiceAccountsManage => "service_accounts.manage",
            DeveloperPermission::ConsoleWebhooksRead => "webhooks.read",
            DeveloperPermission::ConsoleWebhooksManage => "webhooks.manage",
            DeveloperPermission::WebhooksReplay => "webhooks.replay",
            DeveloperPermission::ConsoleLogsRead => "logs.read",
            DeveloperPermission::ConsoleTokensInspect => "tokens.inspect",
            DeveloperPermission::ConsoleHealthChecksRead => "health_checks.read",
            DeveloperPermission::HealthChecksRun => "health_checks.run",
            DeveloperPermission::SandboxUse => "sandbox.use",
            DeveloperPermission::ConsoleRbacManage => "rbac.manage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperRoleParseError {
    literal: String,
}

impl DeveloperRoleParseError {
    pub fn literal(&self) -> &str {
        &self.literal
    }
}

impl TryFrom<&str> for DeveloperRole {
    type Error = DeveloperRoleParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "developer_admin" => Ok(Self::DeveloperAdmin),
            "app_manager" => Ok(Self::AppManager),
            "webhook_manager" => Ok(Self::WebhookManager),
            "log_viewer" => Ok(Self::LogViewer),
            "integration_tester" => Ok(Self::IntegrationTester),
            "docs_viewer" => Ok(Self::DocsViewer),
            literal => Err(DeveloperRoleParseError {
                literal: literal.to_string(),
            }),
        }
    }
}
