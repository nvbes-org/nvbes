import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, ClipboardList } from 'lucide-react';
import { listDeveloperConsoleLogs } from '../developer.api';

export function LogsPage() {
  const logsQuery = useQuery({
    queryKey: ['developer-logs'],
    queryFn: ({ signal }) => listDeveloperConsoleLogs(signal),
    staleTime: 15_000,
  });

  if (logsQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (logsQuery.isError || !logsQuery.data) {
    return <LogsUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <ClipboardList className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Integration logs</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Recent integration events across webhook delivery and replay workflows.
          </p>
        </div>
      </div>
      <div className="overflow-hidden rounded-lg border border-border bg-card">
        {logsQuery.data.map((log) => (
          <article key={log.id} className="border-b border-border p-4 last:border-0">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-sm font-semibold">{log.event_type}</p>
              <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                {log.severity}
              </span>
            </div>
            <p className="mt-2 text-sm text-muted-foreground">{log.message}</p>
          </article>
        ))}
        {logsQuery.data.length === 0 ? (
          <p className="p-6 text-sm text-muted-foreground">No integration logs yet.</p>
        ) : null}
      </div>
    </section>
  );
}

function LogsUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Integration logs unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load integration logs.
      </p>
    </section>
  );
}
