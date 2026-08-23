import { commandSearchTokenValue, parseCommandSearch, RelativeTime } from '@nvbes/web-runtime';
import { CommandSearch, type CommandSearchToken } from '@nvbes/web-ui';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import { listDeveloperLogs, type DeveloperLogsFilter } from '@/developer.api';
import { DeveloperVirtualStack } from './DeveloperVirtualStack';
import { EmptyState, Field, PageHeader, formString, inputClass } from './Portal.shared';

const logSearchTokens: CommandSearchToken[] = [
  { key: 'client', description: 'OAuth client ID', example: 'client:portal' },
  { key: 'event', description: 'Event type', example: 'event:login.failed' },
  { key: 'route', description: 'Route path', example: 'route:/oauth/token' },
  { key: 'since', description: 'Start date', example: 'since:2026-06-01' },
  { key: 'status', description: 'HTTP status', example: 'status:failed' },
  { key: 'tenant', description: 'Tenant ID', example: 'tenant:tenant_123' },
  { key: 'trace', description: 'Trace ID', example: 'trace:abc123' },
  { key: 'user', description: 'User ID', example: 'user:user_123' },
];

export function PortalLogsPage() {
  const [filters, setFilters] = useState<DeveloperLogsFilter>({});
  const [query, setQuery] = useState('');
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
      <PageHeader
        title="Logs"
        body="Filter tenant logs by query tokens, user, client, tenant, event type, and date."
      />
      <form
        className="mb-6 grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-3"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          const commandQuery = parseCommandSearch(query);
          setFilters({
            user_id:
              formString(form, 'user_id').trim() ||
              commandSearchTokenValue(commandQuery, 'user') ||
              undefined,
            client_id:
              formString(form, 'client_id').trim() ||
              commandSearchTokenValue(commandQuery, 'client') ||
              undefined,
            tenant_id:
              formString(form, 'tenant_id').trim() ||
              commandSearchTokenValue(commandQuery, 'tenant') ||
              undefined,
            event_type:
              formString(form, 'event_type').trim() ||
              commandSearchTokenValue(commandQuery, 'event') ||
              commandQuery.text ||
              undefined,
          });
          setFrom(formString(form, 'from'));
          setTo(formString(form, 'to'));
        }}
      >
        <Field label="Query">
          <CommandSearch
            value={query}
            onChange={setQuery}
            tokens={logSearchTokens}
            placeholder='event:login.failed client:"portal"'
          />
        </Field>
        <Field label="User ID">
          <input name="user_id" className={inputClass} />
        </Field>
        <Field label="Client ID">
          <input name="client_id" className={inputClass} />
        </Field>
        <Field label="Tenant ID">
          <input name="tenant_id" className={inputClass} />
        </Field>
        <Field label="Event type">
          <input name="event_type" className={inputClass} />
        </Field>
        <Field label="From">
          <input name="from" type="datetime-local" className={inputClass} />
        </Field>
        <Field label="To">
          <input name="to" type="datetime-local" className={inputClass} />
        </Field>
        <button
          type="submit"
          className="rounded-md border border-border px-4 py-2 text-sm font-medium"
        >
          Apply filters
        </button>
      </form>
      {logs.length ? (
        <DeveloperVirtualStack
          items={logs}
          className="grid max-h-[720px] gap-2 overflow-auto pr-1"
          itemClassName="pb-2"
          estimateSize={78}
          getKey={(log) => log.id}
          renderItem={(log) => (
            <div key={log.id} className="rounded-md border border-border bg-card p-3 text-sm">
              <div className="font-medium">{log.event_type}</div>
              <div className="mt-1 text-xs text-muted-foreground">
                <RelativeTime value={log.created_at} /> · user {log.user_id ?? '-'} · client{' '}
                {log.client_id ?? '-'}
              </div>
            </div>
          )}
        />
      ) : (
        <EmptyState title="No logs" body="No tenant logs match the current filters." />
      )}
    </section>
  );
}
