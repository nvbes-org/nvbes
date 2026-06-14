import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';
import { DeveloperShell } from './layouts/DeveloperShell';
import { ConsentScreenPage } from './pages/ConsentScreenPage';
import { HealthChecksPage } from './pages/HealthChecksPage';
import { MarketplacePage } from './pages/MarketplacePage';
import { OAuthAppsPage } from './pages/OAuthAppsPage';
import { LogsPage } from './pages/LogsPage';
import { SandboxPage } from './pages/SandboxPage';
import { ScopesPage } from './pages/ScopesPage';
import { SecretsPage } from './pages/SecretsPage';
import { ServiceAccountsPage } from './pages/ServiceAccountsPage';
import { TokenDebuggerPage } from './pages/TokenDebuggerPage';
import { WebhooksPage } from './pages/WebhooksPage';

function RootShell() {
  return <Outlet />;
}

function DeveloperHomePage() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <section className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-8">
        <nav className="flex items-center justify-between border-b border-border pb-4">
          <Link to="/" className="text-lg font-semibold">
            nvbes Developers
          </Link>
          <a
            href="/console"
            className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          >
            Open console
          </a>
        </nav>
        <div className="grid flex-1 items-center gap-6 md:grid-cols-[1fr_380px]">
          <div className="space-y-5">
            <p className="text-sm font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Identity integrations
            </p>
            <h1 className="max-w-3xl text-5xl font-semibold tracking-normal">
              Govern OAuth apps, tokens, webhooks, and scopes.
            </h1>
            <p className="max-w-2xl text-base leading-7 text-muted-foreground">
              Manage nvbes Identity integrations from a dedicated developer console.
            </p>
          </div>
          <div className="grid gap-3 rounded-lg border border-border bg-card p-4">
            {['OAuth apps', 'Scope registry', 'Secret rotation', 'Token debugger'].map((label) => (
              <div
                key={label}
                className="rounded-md border border-border bg-background px-4 py-3 text-sm font-medium"
              >
                {label}
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}

const rootRoute = createRootRoute({ component: RootShell });
const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DeveloperHomePage,
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
  homeRoute,
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
