import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';

import { getDeveloperMe } from '@/developer.api';
import { PageHeader } from './Portal.shared';

const cards = [
  { to: '/portal/apps', title: 'Apps', body: 'OAuth client setup and redirect configuration.' },
  { to: '/portal/tokens/inspect', title: 'Tokens', body: 'JWT parsing and backend inspection.' },
  { to: '/portal/oauth/playground', title: 'OAuth', body: 'PKCE URL generation and code exchange.' },
  { to: '/portal/logs', title: 'Logs', body: 'Tenant-scoped activity and audit events.' },
  { to: '/portal/webhooks', title: 'Webhooks', body: 'Identity event subscriptions.' },
  { to: '/portal/roles', title: 'Roles', body: 'Fine-grained developer access model.' },
] as const;

export function PortalOverviewPage() {
  const meQuery = useQuery({
    queryKey: ['developer', 'me'],
    queryFn: ({ signal }) => getDeveloperMe({ signal }),
  });

  return (
    <section>
      <PageHeader
        title="Developer portal"
        body="Manage Identity integrations, inspect tokens, test OAuth flows, and review tenant-scoped logs."
      />
      <div className="mb-6 rounded-md border border-border bg-card p-4 text-sm">
        <span className="text-muted-foreground">Permissions: </span>
        {meQuery.data?.permissions.join(', ') ?? 'Loading'}
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        {cards.map((card) => (
          <Link key={card.to} to={card.to} className="rounded-md border border-border bg-card p-4">
            <h2 className="font-semibold">{card.title}</h2>
            <p className="mt-2 text-sm text-muted-foreground">{card.body}</p>
          </Link>
        ))}
      </div>
    </section>
  );
}
