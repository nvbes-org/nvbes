import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';

function RootShell() {
  return <Outlet />;
}

function HomePage() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <section className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-10">
        <nav className="flex items-center justify-between border-b border-border pb-4">
          <Link to="/" className="font-heading text-lg font-semibold">
            nvbes Developers
          </Link>
          <Link to="/portal" className="text-sm font-medium text-primary">
            Open portal
          </Link>
        </nav>
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
                to="/portal"
                className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
              >
                Open portal preview
              </Link>
              <Link
                to="/"
                className="rounded-md border border-border px-4 py-2 text-sm font-medium"
              >
                View overview
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
    </main>
  );
}

function PortalEntryPreview() {
  return (
    <main className="min-h-screen bg-background px-6 py-8 text-foreground">
      <div className="mx-auto max-w-6xl">
        <h1 className="text-2xl font-semibold">Developer portal</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Connected portal routes are introduced by Tasks 8, 9, and 10 in this plan.
        </p>
      </div>
    </main>
  );
}

const rootRoute = createRootRoute({ component: RootShell });

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomePage,
});

const portalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/portal',
  component: PortalEntryPreview,
});

const routeTree = rootRoute.addChildren([homeRoute, portalRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
