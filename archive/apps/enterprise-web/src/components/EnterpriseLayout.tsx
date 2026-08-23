import { useQuery } from '@tanstack/react-query';
import { Outlet } from '@tanstack/react-router';
import { enterpriseContextQueryOptions } from '../enterprise.queries';
import { EnterpriseSidebar } from './EnterpriseSidebar';

export function EnterpriseLayout() {
  const { data: context } = useQuery(enterpriseContextQueryOptions());

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="hidden w-64 shrink-0 border-r border-border bg-card md:flex md:flex-col">
        <EnterpriseSidebar />
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-card px-4 md:hidden">
          <div className="min-w-0">
            <p className="truncate text-sm font-heading font-semibold">nvbes Enterprise</p>
            <p className="truncate text-[0.7rem] text-muted-foreground">
              {context?.organization_id ? 'Org admin' : 'Tenant admin'}
            </p>
          </div>
        </header>

        <div className="shrink-0 border-b border-border bg-background md:hidden">
          <EnterpriseSidebar variant="mobile" />
        </div>

        <main className="flex-1 overflow-auto">
          <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-4 py-6 md:px-8 md:py-8">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
