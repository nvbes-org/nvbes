import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';

import { DeveloperPortalLayout } from './layouts/DeveloperPortalLayout';
import { DeveloperPublicLayout } from './layouts/DeveloperPublicLayout';
import { ApiReferencePage } from './pages/ApiReferencePage';
import { DeveloperHomePage } from './pages/DeveloperHomePage';
import { PortalAppDetailPage } from './pages/PortalAppDetailPage';
import { PortalAppsPage } from './pages/PortalAppsPage';
import { PortalLogsPage } from './pages/PortalLogsPage';
import { PortalOAuthPlaygroundPage } from './pages/PortalOAuthPlaygroundPage';
import { PortalOverviewPage } from './pages/PortalOverviewPage';
import { PortalRolesPage } from './pages/PortalRolesPage';
import { PortalTokenInspectorPage } from './pages/PortalTokenInspectorPage';
import { PortalWebhooksPage } from './pages/PortalWebhooksPage';
import { QuickstartPage } from './pages/QuickstartPage';

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

const routeTree = rootRoute.addChildren([
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
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
