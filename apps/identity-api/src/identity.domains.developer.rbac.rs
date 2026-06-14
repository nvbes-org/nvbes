#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperRole {
    DeveloperAdmin,
    AppManager,
    WebhookManager,
    LogViewer,
    IntegrationTester,
    DocsViewer,
}

impl DeveloperRole {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::DeveloperAdmin => "developer_admin",
            Self::AppManager => "app_manager",
            Self::WebhookManager => "webhook_manager",
            Self::LogViewer => "log_viewer",
            Self::IntegrationTester => "integration_tester",
            Self::DocsViewer => "docs_viewer",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperPermission {
    DocsRead,
    AppsRead,
    AppsCreate,
    AppsUpdate,
    AppsRevoke,
    MarketplaceRead,
    MarketplaceSubmit,
    MarketplaceReview,
    ScopesRead,
    ScopesManage,
    SecretsRotate,
    ServiceAccountsRead,
    ServiceAccountsManage,
    WebhooksRead,
    WebhooksManage,
    WebhooksReplay,
    LogsRead,
    TokensInspect,
    HealthChecksRead,
    HealthChecksRun,
    SandboxUse,
    RbacManage,
}

impl DeveloperPermission {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::DocsRead => "docs.read",
            Self::AppsRead => "apps.read",
            Self::AppsCreate => "apps.create",
            Self::AppsUpdate => "apps.update",
            Self::AppsRevoke => "apps.revoke",
            Self::MarketplaceRead => "marketplace.read",
            Self::MarketplaceSubmit => "marketplace.submit",
            Self::MarketplaceReview => "marketplace.review",
            Self::ScopesRead => "scopes.read",
            Self::ScopesManage => "scopes.manage",
            Self::SecretsRotate => "secrets.rotate",
            Self::ServiceAccountsRead => "service_accounts.read",
            Self::ServiceAccountsManage => "service_accounts.manage",
            Self::WebhooksRead => "webhooks.read",
            Self::WebhooksManage => "webhooks.manage",
            Self::WebhooksReplay => "webhooks.replay",
            Self::LogsRead => "logs.read",
            Self::TokensInspect => "tokens.inspect",
            Self::HealthChecksRead => "health_checks.read",
            Self::HealthChecksRun => "health_checks.run",
            Self::SandboxUse => "sandbox.use",
            Self::RbacManage => "rbac.manage",
        }
    }
}

pub fn permissions_for_role(role: DeveloperRole) -> &'static [DeveloperPermission] {
    use DeveloperPermission::*;

    match role {
        DeveloperRole::DeveloperAdmin => &[
            DocsRead,
            AppsRead,
            AppsCreate,
            AppsUpdate,
            AppsRevoke,
            MarketplaceRead,
            MarketplaceSubmit,
            MarketplaceReview,
            ScopesRead,
            ScopesManage,
            SecretsRotate,
            ServiceAccountsRead,
            ServiceAccountsManage,
            WebhooksRead,
            WebhooksManage,
            WebhooksReplay,
            LogsRead,
            TokensInspect,
            HealthChecksRead,
            HealthChecksRun,
            SandboxUse,
            RbacManage,
        ],
        DeveloperRole::AppManager => &[
            DocsRead,
            AppsRead,
            AppsCreate,
            AppsUpdate,
            AppsRevoke,
            MarketplaceRead,
            MarketplaceSubmit,
            ScopesRead,
            SecretsRotate,
            ServiceAccountsRead,
            ServiceAccountsManage,
            HealthChecksRead,
        ],
        DeveloperRole::WebhookManager => &[
            DocsRead,
            WebhooksRead,
            WebhooksManage,
            WebhooksReplay,
            LogsRead,
            HealthChecksRead,
            HealthChecksRun,
        ],
        DeveloperRole::LogViewer => &[DocsRead, AppsRead, WebhooksRead, LogsRead, HealthChecksRead],
        DeveloperRole::IntegrationTester => &[
            DocsRead,
            AppsRead,
            MarketplaceRead,
            ScopesRead,
            ServiceAccountsRead,
            WebhooksRead,
            LogsRead,
            TokensInspect,
            HealthChecksRead,
            HealthChecksRun,
            SandboxUse,
        ],
        DeveloperRole::DocsViewer => &[DocsRead],
    }
}
