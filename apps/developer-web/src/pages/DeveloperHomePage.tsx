import { Link } from '@tanstack/react-router';

export function DeveloperHomePage() {
  return (
    <section className="mx-auto flex min-h-[calc(100vh-3.5rem)] w-full max-w-6xl flex-col gap-8 px-6 py-10">
      <div className="flex min-h-[44vh] flex-col justify-center gap-5">
        <p className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          Identity platform
        </p>
        <h1 className="max-w-3xl text-4xl font-semibold tracking-normal md:text-6xl">
          nvbes Developers
        </h1>
        <p className="max-w-2xl text-base leading-7 text-muted-foreground">
          Build secure OAuth apps with nvbes Identity. Configure clients, redirects, webhooks, logs,
          token inspection, SDKs, and quickstarts from one developer console.
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

      <div className="grid gap-3 md:grid-cols-4">
        {[
          ['OAuth apps', 'Create clients, copy IDs, rotate credentials, and manage redirects.'],
          ['Token inspector', 'Validate access tokens internally without exposing secrets.'],
          ['Webhooks', 'Subscribe to identity events with signing-ready endpoints.'],
          ['Filtered logs', 'Search activity by user, client, tenant, or event type.'],
        ].map(([label, description]) => (
          <div key={label} className="rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-semibold">{label}</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">{description}</p>
          </div>
        ))}
      </div>

      <div className="grid gap-3 rounded-md border border-border bg-background p-3 text-xs text-muted-foreground">
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Portal activity
          </p>
          <span>Live identity control plane</span>
        </div>
        {[
          'client.created / tenant:acme / app:customer-portal',
          'login.failed / user:usr_42 / risk:medium',
          'session.revoked / user:usr_17 / source:admin',
        ].map((event) => (
          <div key={event} className="rounded-sm bg-muted px-3 py-2 font-mono">
            {event}
          </div>
        ))}
      </div>
    </section>
  );
}
