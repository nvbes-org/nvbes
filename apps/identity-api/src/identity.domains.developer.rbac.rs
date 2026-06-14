#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperRole {
    DeveloperAdmin,
    AppManager,
    WebhookManager,
    LogViewer,
    IntegrationTester,
    DocsViewer,
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

pub fn permissions_for_role(role: DeveloperRole) -> Vec<DeveloperPermission> {
    use DeveloperPermission::*;

    match role {
        DeveloperRole::DeveloperAdmin => vec![
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
        DeveloperRole::AppManager => vec![
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
        DeveloperRole::WebhookManager => vec![
            DocsRead,
            WebhooksRead,
            WebhooksManage,
            WebhooksReplay,
            LogsRead,
            HealthChecksRead,
            HealthChecksRun,
        ],
        DeveloperRole::LogViewer => {
            vec![DocsRead, AppsRead, WebhooksRead, LogsRead, HealthChecksRead]
        }
        DeveloperRole::IntegrationTester => vec![
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
        DeveloperRole::DocsViewer => vec![DocsRead],
    }
}
