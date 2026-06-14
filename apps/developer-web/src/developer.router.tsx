import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';

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
          <Link to="/console" className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground">
            Open console
          </Link>
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
              <div key={label} className="rounded-md border border-border bg-background px-4 py-3 text-sm font-medium">
                {label}
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}

function ConsolePreviewPage() {
  return (
    <main className="min-h-screen bg-background px-6 py-8 text-foreground">
      <div className="mx-auto max-w-6xl">
        <h1 className="text-2xl font-semibold">Developer Console</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Console routes connect to `/api/v1/developer` in later tasks.
        </p>
      </div>
    </main>
  );
}

const rootRoute = createRootRoute({ component: RootShell });
const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: DeveloperHomePage });
const consoleRoute = createRoute({ getParentRoute: () => rootRoute, path: '/console', component: ConsolePreviewPage });
const routeTree = rootRoute.addChildren([homeRoute, consoleRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
