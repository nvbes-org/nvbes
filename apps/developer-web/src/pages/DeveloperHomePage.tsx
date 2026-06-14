import { Link } from '@tanstack/react-router';

export function DeveloperHomePage() {
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
