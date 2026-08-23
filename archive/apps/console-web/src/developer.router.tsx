import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';

import { DeveloperPortalLayout } from './layouts/DeveloperPortalLayout';
import { DeveloperPublicLayout } from './layouts/DeveloperPublicLayout';
import { DeveloperShell } from './layouts/DeveloperShell';
import { ApiReferencePage } from './pages/ApiReferencePage';
import { ConsentScreenPage } from './pages/ConsentScreenPage';
import { DeveloperCallbackPage } from './pages/DeveloperCallbackPage';
import { DeveloperHomePage } from './pages/DeveloperHomePage';
import { HealthChecksPage } from './pages/HealthChecksPage';
import { LogsPage } from './pages/LogsPage';
import { MarketplacePage } from './pages/MarketplacePage';
import { OAuthAppsPage } from './pages/OAuthAppsPage';
import { PortalAppDetailPage } from './pages/PortalAppDetailPage';
import { PortalAppsPage } from './pages/PortalAppsPage';
import { PortalLogsPage } from './pages/PortalLogsPage';
import { PortalOAuthPlaygroundPage } from './pages/PortalOAuthPlaygroundPage';
import { PortalOverviewPage } from './pages/PortalOverviewPage';
import { PortalRolesPage } from './pages/PortalRolesPage';
import { PortalTokenInspectorPage } from './pages/PortalTokenInspectorPage';
import { PortalWebhooksPage } from './pages/PortalWebhooksPage';
import { QuickstartPage } from './pages/QuickstartPage';
import { SandboxPage } from './pages/SandboxPage';
import { ScopesPage } from './pages/ScopesPage';
import { SecretsPage } from './pages/SecretsPage';
import { ServiceAccountsPage } from './pages/ServiceAccountsPage';
import { TokenDebuggerPage } from './pages/TokenDebuggerPage';
import { WebhooksPage } from './pages/WebhooksPage';

function RootShell() {
  return <Outlet />;
}

const rootRoute = createRootRoute({ component: RootShell });

const publicRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'public',
  component: DeveloperPublicLayout,
});

const homeRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/',
  component: DeveloperHomePage,
});

const reactQuickstartRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/quickstarts/react',
  component: () => <QuickstartPage quickstartKey="react" />,
});

const rustQuickstartRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/quickstarts/rust-axum',
  component: () => <QuickstartPage quickstartKey="rust-axum" />,
});

const nodeQuickstartRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/quickstarts/node',
  component: () => <QuickstartPage quickstartKey="node" />,
});

const curlQuickstartRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/quickstarts/curl',
  component: () => <QuickstartPage quickstartKey="curl" />,
});

const apiReferenceRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/api-reference',
  component: ApiReferencePage,
});

const callbackRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/callback',
  component: DeveloperCallbackPage,
});

const portalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/portal',
  component: DeveloperPortalLayout,
});

const portalOverviewRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: '/',
  component: PortalOverviewPage,
});

const portalAppsRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'apps',
  component: PortalAppsPage,
});

const portalAppDetailRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'apps/$clientId',
  component: PortalAppDetailPage,
});

const portalTokensRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'tokens/inspect',
  component: PortalTokenInspectorPage,
});

const portalOAuthRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'oauth/playground',
  component: PortalOAuthPlaygroundPage,
});

const portalLogsRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'logs',
  component: PortalLogsPage,
});

const portalWebhooksRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'webhooks',
  component: PortalWebhooksPage,
});

const portalRolesRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'roles',
  component: PortalRolesPage,
});

const consoleRootRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/console',
  component: DeveloperShell,
});

const consoleOAuthClientsRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/oauth-clients',
  component: OAuthAppsPage,
});

const consoleMarketplaceRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/marketplace',
  component: MarketplacePage,
});

const consoleScopesRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/scopes',
  component: ScopesPage,
});

const consoleSecretsRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/secrets',
  component: SecretsPage,
});

const consoleServiceAccountsRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/service-accounts',
  component: ServiceAccountsPage,
});

const consoleWebhooksRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/webhooks',
  component: WebhooksPage,
});

const consoleLogsRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/logs',
  component: LogsPage,
});

const consoleConsentRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/consent',
  component: ConsentScreenPage,
});

const consoleSandboxRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/sandbox',
  component: SandboxPage,
});

const consoleTokensRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/tokens',
  component: TokenDebuggerPage,
});

const consoleHealthRoute = createRoute({
  getParentRoute: () => consoleRootRoute,
  path: '/health',
  component: HealthChecksPage,
});

const routeTree = rootRoute.addChildren([
  callbackRoute,
  publicRoute.addChildren([
    homeRoute,
    reactQuickstartRoute,
    rustQuickstartRoute,
    nodeQuickstartRoute,
    curlQuickstartRoute,
    apiReferenceRoute,
  ]),
  portalRoute.addChildren([
    portalOverviewRoute,
    portalAppsRoute,
    portalAppDetailRoute,
    portalTokensRoute,
    portalOAuthRoute,
    portalLogsRoute,
    portalWebhooksRoute,
    portalRolesRoute,
  ]),
  consoleRootRoute.addChildren([
    consoleOAuthClientsRoute,
    consoleMarketplaceRoute,
    consoleScopesRoute,
    consoleSecretsRoute,
    consoleServiceAccountsRoute,
    consoleWebhooksRoute,
    consoleLogsRoute,
    consoleConsentRoute,
    consoleSandboxRoute,
    consoleTokensRoute,
    consoleHealthRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
