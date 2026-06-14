import { useQuery } from '@tanstack/react-query';

import { getDeveloperMe } from '@/developer.api';
import { EmptyState, PageHeader } from './Portal.shared';

export function PortalRolesPage() {
  const meQuery = useQuery({
    queryKey: ['developer', 'me'],
    queryFn: ({ signal }) => getDeveloperMe({ signal }),
  });

  return (
    <section>
      <PageHeader title="Developer roles" body="Review the fine-grained roles and permissions active for this tenant session." />
      {meQuery.data ? (
        <div className="grid gap-4 md:grid-cols-2">
          <div className="rounded-md border border-border bg-card p-4">
            <h2 className="font-semibold">Roles</h2>
            <ul className="mt-3 grid gap-2 text-sm">
              {meQuery.data.roles.map((role) => <li key={role}>{role}</li>)}
            </ul>
          </div>
          <div className="rounded-md border border-border bg-card p-4">
            <h2 className="font-semibold">Permissions</h2>
            <ul className="mt-3 grid gap-2 text-sm">
              {meQuery.data.permissions.map((permission) => <li key={permission}>{permission}</li>)}
            </ul>
          </div>
        </div>
      ) : (
        <EmptyState title="No developer context" body="Developer roles load after the portal session is authenticated." />
      )}
    </section>
  );
}
