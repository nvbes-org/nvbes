import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';

import { DeveloperPortalLayout } from './layouts/DeveloperPortalLayout';
import { DeveloperPublicLayout } from './layouts/DeveloperPublicLayout';

function RootShell() {
  return <Outlet />;
}

function HomePage() {
  return (
    <section className="mx-auto flex min-h-[calc(100vh-3.5rem)] w-full max-w-6xl flex-col gap-8 px-6 py-10">
        <div className="grid gap-6 md:grid-cols-[1.1fr_0.9fr]">
          <div className="flex flex-col justify-center gap-5">
            <p className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              Identity platform
            </p>
            <h1 className="max-w-3xl text-4xl font-semibold tracking-normal md:text-6xl">
              Build secure OAuth apps with nvbes Identity.
            </h1>
            <p className="max-w-2xl text-base leading-7 text-muted-foreground">
              Configure clients, redirects, webhooks, logs, token inspection, SDKs, and quickstarts
              from one developer console.
            </p>
            <div className="flex flex-wrap gap-3">
              <Link
                to="/portal/apps"
                className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
              >
                Open portal
              </Link>
              <Link
                to="/quickstarts/react"
                className="rounded-md border border-border px-4 py-2 text-sm font-medium"
              >
                React quickstart
              </Link>
            </div>
          </div>
          <div className="grid content-start gap-3 rounded-lg border border-border bg-card p-4">
            {['OAuth apps', 'Token inspector', 'Webhooks', 'Filtered logs'].map((label) => (
              <div
                key={label}
                className="rounded-md border border-border bg-background p-4 text-sm font-medium"
              >
                {label}
              </div>
            ))}
          </div>
        </div>
      </section>
  );
}

function PublicPlaceholder({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto flex min-h-[calc(100vh-3.5rem)] max-w-5xl flex-col justify-center px-6 py-10">
      <h1 className="text-3xl font-semibold md:text-5xl">{title}</h1>
      <p className="mt-4 max-w-2xl text-sm leading-6 text-muted-foreground md:text-base">
        {body}
      </p>
    </section>
  );
}

function PortalPlaceholder({ title, body }: { title: string; body: string }) {
  return (
    <section className="rounded-md border border-border bg-card p-6">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{body}</p>
    </section>
  );
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
  component: HomePage,
});

const reactQuickstartRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/quickstarts/react',
  component: () => (
    <PublicPlaceholder
      title="React quickstart"
      body="Install the nvbes Identity SDK, configure your OAuth client, and wire PKCE login from a React app."
    />
  ),
});

const apiReferenceRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/api-reference',
  component: () => (
    <PublicPlaceholder
      title="Identity API reference"
      body="Browse the generated OpenAPI contract for developer apps, webhooks, token inspection, OAuth playground, and logs."
    />
  ),
});

const portalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/portal',
  component: DeveloperPortalLayout,
});

const portalOverviewRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: '/',
  component: () => (
    <PortalPlaceholder
      title="Developer portal"
      body="Manage Identity integrations, inspect tokens, test OAuth flows, and review tenant-scoped logs."
    />
  ),
});

const portalAppsRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'apps',
  component: () => (
    <PortalPlaceholder
      title="OAuth apps"
      body="Create apps, copy client IDs, and manage redirect URI configuration."
    />
  ),
});

const portalTokensRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'tokens/inspect',
  component: () => (
    <PortalPlaceholder
      title="Token inspector"
      body="Inspect access tokens without persisting raw token bodies."
    />
  ),
});

const portalOAuthRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'oauth/playground',
  component: () => (
    <PortalPlaceholder
      title="OAuth playground"
      body="Exchange authorization codes and validate PKCE integration settings."
    />
  ),
});

const portalLogsRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'logs',
  component: () => (
    <PortalPlaceholder
      title="Logs"
      body="Filter tenant logs by user, client, tenant, and event type."
    />
  ),
});

const portalWebhooksRoute = createRoute({
  getParentRoute: () => portalRoute,
  path: 'webhooks',
  component: () => (
    <PortalPlaceholder
      title="Webhooks"
      body="Subscribe endpoints to Identity events: user.created, login.failed, session.revoked, and client.created."
    />
  ),
});

const routeTree = rootRoute.addChildren([
  publicRoute.addChildren([homeRoute, reactQuickstartRoute, apiReferenceRoute]),
  portalRoute.addChildren([
    portalOverviewRoute,
    portalAppsRoute,
    portalTokensRoute,
    portalOAuthRoute,
    portalLogsRoute,
    portalWebhooksRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
