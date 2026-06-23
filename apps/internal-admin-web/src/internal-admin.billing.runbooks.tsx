import { useQuery } from '@tanstack/react-query';
import { LifeBuoy } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { listBillingRunbooks } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function BillingRunbooks({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const runbooks = useQuery({
    queryKey: ['billing-runbooks'],
    queryFn: () => listBillingRunbooks(credentials),
    enabled: !disabled,
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="runbooks">
      <div className="mb-4 flex items-center gap-3">
        <div className="bg-secondary text-secondary-foreground flex size-9 items-center justify-center rounded-md">
          <LifeBuoy className="size-4" />
        </div>
        <div>
          <h2 className="text-sm font-semibold">Runbooks billing</h2>
          <p className="text-muted-foreground text-xs">
            Procedures operateur exposees par l'API interne.
          </p>
        </div>
      </div>
      <div className="space-y-3">
        {disabled ? (
          <LockedState label="Connecte un contexte operateur pour charger les runbooks internes." />
        ) : null}
        {runbooks.isLoading ? <Skeleton className="h-24 w-full" /> : null}
        {(runbooks.data ?? []).map((runbook) => (
          <article className="bg-muted/40 rounded-md p-3" key={runbook.id}>
            <div className="mb-2 flex items-center justify-between gap-3">
              <h3 className="text-sm font-medium">{runbook.title}</h3>
              <Badge variant={runbook.severity === 'critical' ? 'destructive' : 'secondary'}>
                {runbook.severity}
              </Badge>
            </div>
            <ol className="text-muted-foreground list-decimal space-y-1 pl-4 text-xs">
              {runbook.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          </article>
        ))}
        {runbooks.error ? (
          <p className="text-destructive text-xs">
            {runbooks.error instanceof Error ? runbooks.error.message : 'Runbooks unavailable'}
          </p>
        ) : null}
      </div>
    </section>
  );
}
