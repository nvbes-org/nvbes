import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, Outlet, useLocation } from '@tanstack/react-router';
import {
  Activity,
  Bot,
  Boxes,
  Bug,
  ClipboardList,
  Gauge,
  KeyRound,
  Library,
  LogOut,
  RotateCcwKey,
  ShieldCheck,
  Store,
  Webhook,
} from 'lucide-react';
import { getDeveloperContext } from '../developer.api';
import { canUseDeveloperPermission } from '../developer.permissions';
import {
  isDeveloperAuthError,
  logoutDeveloperSession,
  redirectToDeveloperLogin,
} from '../developer.session';
import type { DeveloperPermission } from '../developer.schemas';
import { OverviewPage } from '../pages/OverviewPage';

type NavigationItem = {
  label: string;
  to: string;
  permission: DeveloperPermission;
  icon: React.ComponentType<{ className?: string }>;
};

const navigationItems: NavigationItem[] = [
  { label: 'Overview', to: '/console', permission: 'docs.read', icon: Gauge },
  { label: 'OAuth clients', to: '/console/oauth-clients', permission: 'apps.read', icon: KeyRound },
  { label: 'Marketplace', to: '/console/marketplace', permission: 'marketplace.read', icon: Store },
  {
    label: 'Consent screens',
    to: '/console/consent',
    permission: 'apps.update',
    icon: ShieldCheck,
  },
  { label: 'Scope registry', to: '/console/scopes', permission: 'scopes.read', icon: Library },
  {
    label: 'Secret rotation',
    to: '/console/secrets',
    permission: 'secrets.rotate',
    icon: RotateCcwKey,
  },
  {
    label: 'Service accounts',
    to: '/console/service-accounts',
    permission: 'service_accounts.read',
    icon: Bot,
  },
  { label: 'Webhooks', to: '/console/webhooks', permission: 'webhooks.read', icon: Webhook },
  { label: 'Sandbox tenants', to: '/console/sandbox', permission: 'sandbox.use', icon: Boxes },
  { label: 'Token debugger', to: '/console/tokens', permission: 'tokens.inspect', icon: Bug },
  {
    label: 'Health checks',
    to: '/console/health',
    permission: 'health_checks.read',
    icon: Activity,
  },
  { label: 'Audit logs', to: '/console/logs', permission: 'logs.read', icon: ClipboardList },
];

export function DeveloperShell() {
  const location = useLocation();
  const queryClient = useQueryClient();
  const isConsoleRoot = location.pathname === '/console' || location.pathname === '/console/';
  const contextQuery = useQuery({
    queryKey: ['developer-context'],
    queryFn: ({ signal }) => getDeveloperContext(signal),
    staleTime: 60_000,
    retry: false,
  });
  const logoutMutation = useMutation({
    mutationFn: logoutDeveloperSession,
    onSettled: () => {
      queryClient.removeQueries({ queryKey: ['developer-context'] });
      window.location.assign('/');
    },
  });

  const context = contextQuery.data;

  if (contextQuery.isError && isDeveloperAuthError(contextQuery.error)) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
        <div className="w-full max-w-md rounded-md border border-border bg-card p-6">
          <p className="text-xs font-medium uppercase text-muted-foreground">Developer Console</p>
          <h1 className="mt-2 text-lg font-semibold">Sign in required</h1>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            Use an Identity session with developer console permissions to continue.
          </p>
          <button
            type="button"
            onClick={() => redirectToDeveloperLogin()}
            className="mt-5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          >
            Sign in
          </button>
        </div>
      </main>
    );
  }

  if (contextQuery.isLoading) {
    return (
      <ConsoleSessionState title="Loading session" message="Checking developer console access." />
    );
  }

  if (contextQuery.isError || !context) {
    return (
      <ConsoleSessionState
        title="Developer context unavailable"
        message="The console could not load tenant-scoped Identity data."
      />
    );
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <aside className="fixed inset-y-0 left-0 hidden w-72 border-r border-border bg-card lg:block">
        <div className="flex h-full flex-col">
          <div className="border-b border-border px-5 py-4">
            <Link to="/" className="block text-base font-semibold">
              nvbes Developers
            </Link>
            <p className="mt-1 text-xs text-muted-foreground">Identity Developer Console</p>
          </div>
          <nav className="flex-1 space-y-1 overflow-y-auto px-3 py-4">
            {navigationItems.map((item) => {
              const Icon = item.icon;
              const isActive = location.pathname === item.to;
              const isAllowed = context
                ? canUseDeveloperPermission(context, item.permission)
                : true;

              return (
                <Link
                  key={item.to}
                  to={item.to}
                  aria-disabled={!isAllowed}
                  className={[
                    'flex min-h-10 items-center gap-3 rounded-md px-3 text-sm font-medium transition-colors',
                    isActive
                      ? 'bg-primary text-primary-foreground'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                    isAllowed ? '' : 'pointer-events-none opacity-45',
                  ].join(' ')}
                >
                  <Icon className="h-4 w-4 shrink-0" />
                  <span className="truncate">{item.label}</span>
                </Link>
              );
            })}
          </nav>
          <div className="border-t border-border px-5 py-4">
            <p className="truncate text-sm font-medium">
              {contextQuery.isLoading ? 'Loading session' : context?.displayName || 'No session'}
            </p>
            <p className="truncate text-xs text-muted-foreground">
              {context?.email || 'Sign in required'}
            </p>
            {context ? (
              <button
                type="button"
                onClick={() => logoutMutation.mutate()}
                disabled={logoutMutation.isPending}
                className="mt-3 flex w-full items-center gap-2 rounded-md px-0 py-1.5 text-sm text-muted-foreground hover:text-foreground disabled:opacity-50"
              >
                <LogOut className="h-4 w-4" />
                {logoutMutation.isPending ? 'Signing out...' : 'Sign out'}
              </button>
            ) : null}
          </div>
        </div>
      </aside>

      <div className="lg:pl-72">
        <header className="sticky top-0 z-10 border-b border-border bg-background/95 px-4 py-3 backdrop-blur lg:px-8">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-xs font-medium uppercase text-muted-foreground">
                Developer Console
              </p>
              <h1 className="text-lg font-semibold">Identity integrations</h1>
            </div>
            <div className="hidden items-center gap-2 rounded-md border border-border bg-card px-3 py-2 text-xs text-muted-foreground sm:flex">
              <Activity className="h-4 w-4 text-primary" />
              API context
            </div>
          </div>
        </header>
        <main className="px-4 py-6 lg:px-8">{isConsoleRoot ? <OverviewPage /> : <Outlet />}</main>
      </div>
    </div>
  );
}

function ConsoleSessionState({ title, message }: { title: string; message: string }) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <div className="w-full max-w-md rounded-md border border-border bg-card p-6">
        <p className="text-xs font-medium uppercase text-muted-foreground">Developer Console</p>
        <h1 className="mt-2 text-lg font-semibold">{title}</h1>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">{message}</p>
      </div>
    </main>
  );
}
