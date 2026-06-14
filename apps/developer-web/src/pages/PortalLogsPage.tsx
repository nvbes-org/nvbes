import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import { listDeveloperLogs, type DeveloperLogsFilter } from '@/developer.api';
import { EmptyState, Field, PageHeader, formString, inputClass } from './Portal.shared';

export function PortalLogsPage() {
  const [filters, setFilters] = useState<DeveloperLogsFilter>({});
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const logsQuery = useQuery({
    queryKey: ['developer', 'logs', filters],
    queryFn: ({ signal }) => listDeveloperLogs(filters, { signal }),
  });
  const logs = (logsQuery.data ?? []).filter((log) => {
    const time = new Date(log.created_at).getTime();
    return (!from || time >= new Date(from).getTime()) && (!to || time <= new Date(to).getTime());
  });

  return (
    <section>
      <PageHeader title="Logs" body="Filter tenant logs by user, client, tenant, event type, and date." />
      <form
        className="mb-6 grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-3"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          setFilters({
            user_id: formString(form, 'user_id').trim() || undefined,
            client_id: formString(form, 'client_id').trim() || undefined,
            tenant_id: formString(form, 'tenant_id').trim() || undefined,
            event_type: formString(form, 'event_type').trim() || undefined,
          });
          setFrom(formString(form, 'from'));
          setTo(formString(form, 'to'));
        }}
      >
        <Field label="User ID"><input name="user_id" className={inputClass} /></Field>
        <Field label="Client ID"><input name="client_id" className={inputClass} /></Field>
        <Field label="Tenant ID"><input name="tenant_id" className={inputClass} /></Field>
        <Field label="Event type"><input name="event_type" className={inputClass} /></Field>
        <Field label="From"><input name="from" type="datetime-local" className={inputClass} /></Field>
        <Field label="To"><input name="to" type="datetime-local" className={inputClass} /></Field>
        <button type="submit" className="rounded-md border border-border px-4 py-2 text-sm font-medium">
          Apply filters
        </button>
      </form>
      {logs.length ? (
        <div className="grid gap-2">
          {logs.map((log) => (
            <div key={log.id} className="rounded-md border border-border bg-card p-3 text-sm">
              <div className="font-medium">{log.event_type}</div>
              <div className="mt-1 text-xs text-muted-foreground">
                {log.created_at} · user {log.user_id ?? '-'} · client {log.client_id ?? '-'}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState title="No logs" body="No tenant logs match the current filters." />
      )}
    </section>
  );
}
