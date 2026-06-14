import { useQuery } from '@tanstack/react-query';
import { Link, Outlet } from '@tanstack/react-router';
import {
  AppWindow,
  FileJson,
  KeyRound,
  ListFilter,
  RadioTower,
  ShieldCheck,
  Webhook,
} from 'lucide-react';

import { getDeveloperMe } from '@/developer.api';

const navItems = [
  { to: '/portal/apps', label: 'Apps', icon: AppWindow },
  { to: '/portal/tokens/inspect', label: 'Tokens', icon: KeyRound },
  { to: '/portal/oauth/playground', label: 'OAuth', icon: RadioTower },
  { to: '/portal/logs', label: 'Logs', icon: ListFilter },
  { to: '/portal/webhooks', label: 'Webhooks', icon: Webhook },
  { to: '/portal/roles', label: 'Roles', icon: ShieldCheck },
  { to: '/api-reference', label: 'OpenAPI', icon: FileJson },
] as const;

export function DeveloperPortalLayout() {
  const meQuery = useQuery({
    queryKey: ['developer', 'me'],
    queryFn: ({ signal }) => getDeveloperMe({ signal }),
    retry: false,
  });

  if (meQuery.isLoading) {
    return (
      <main className="min-h-screen bg-background p-6 text-sm text-muted-foreground">
        Loading portal...
      </main>
    );
  }

  if (meQuery.isError || !meQuery.data) {
    return (
      <main className="min-h-screen bg-background p-6">
        <div className="rounded-md border border-border bg-card p-6">
          <h1 className="text-lg font-semibold">Developer access required</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Sign in with a tenant account that has developer portal permissions.
          </p>
        </div>
      </main>
    );
  }

  return (
    <div className="grid min-h-screen grid-cols-1 bg-background text-foreground md:grid-cols-[240px_1fr]">
      <aside className="border-b border-border bg-card md:border-r md:border-b-0">
        <div className="border-b border-border px-4 py-4">
          <Link to="/" className="font-heading text-base font-semibold">
            nvbes Developers
          </Link>
          <p className="mt-1 text-xs text-muted-foreground">Tenant {meQuery.data.tenant_id}</p>
        </div>
        <nav className="flex gap-1 overflow-x-auto px-2 py-3 md:flex-col">
          {navItems.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <item.icon className="size-4" />
              {item.label}
            </Link>
          ))}
        </nav>
      </aside>
      <main className="min-w-0 overflow-auto">
        <div className="mx-auto w-full max-w-7xl px-4 py-6 md:px-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
