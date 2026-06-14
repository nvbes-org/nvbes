import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Boxes } from 'lucide-react';
import { useState } from 'react';
import {
  getDeveloperSandbox,
  resetDeveloperSandbox,
  upsertDeveloperSandbox,
} from '../developer.api';

export function SandboxPage() {
  const queryClient = useQueryClient();
  const [dataProfile, setDataProfile] = useState('minimal');
  const sandboxQuery = useQuery({
    queryKey: ['developer-sandbox'],
    queryFn: ({ signal }) => getDeveloperSandbox(signal),
    staleTime: 30_000,
  });
  const upsertMutation = useMutation({
    mutationFn: upsertDeveloperSandbox,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['developer-sandbox'] }),
  });
  const resetMutation = useMutation({
    mutationFn: resetDeveloperSandbox,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['developer-sandbox'] }),
  });

  if (sandboxQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (sandboxQuery.isError) {
    return <SandboxUnavailable />;
  }

  const sandbox = sandboxQuery.data;

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Boxes className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Sandbox tenant</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Isolated tenant environment for client integration tests.
          </p>
        </div>
      </div>
      <div className="grid gap-4 lg:grid-cols-[360px_1fr]">
        <form
          className="rounded-lg border border-border bg-card p-5"
          onSubmit={(event) => {
            event.preventDefault();
            upsertMutation.mutate(dataProfile);
          }}
        >
          <label className="grid gap-2 text-sm font-medium">
            Data profile
            <select
              className="h-10 rounded-md border border-input bg-background px-3 text-sm"
              value={dataProfile}
              onChange={(event) => setDataProfile(event.currentTarget.value)}
            >
              <option value="minimal">minimal</option>
              <option value="oauth">oauth</option>
              <option value="full">full</option>
            </select>
          </label>
          <div className="mt-4 flex flex-wrap gap-2">
            <button
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
              disabled={upsertMutation.isPending}
              type="submit"
            >
              {sandbox ? 'Update sandbox' : 'Create sandbox'}
            </button>
            <button
              className="rounded-md border border-border px-4 py-2 text-sm font-medium disabled:opacity-50"
              disabled={!sandbox || resetMutation.isPending}
              type="button"
              onClick={() => resetMutation.mutate()}
            >
              Reset data
            </button>
          </div>
        </form>
        <article className="rounded-lg border border-border bg-card p-5">
          {sandbox ? (
            <dl className="grid gap-3 text-sm md:grid-cols-2">
              <Field label="Name" value={sandbox.sandbox_name} />
              <Field label="Slug" value={sandbox.sandbox_slug} />
              <Field label="Status" value={sandbox.status} />
              <Field label="Profile" value={sandbox.data_profile} />
              <Field label="Sandbox tenant" value={sandbox.sandbox_tenant_id} />
              <Field label="Updated" value={sandbox.updated_at} />
            </dl>
          ) : (
            <p className="text-sm text-muted-foreground">No sandbox tenant exists.</p>
          )}
        </article>
      </div>
    </section>
  );
}

function SandboxUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Sandbox unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load sandbox state.
      </p>
    </section>
  );
}

function Field(props: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium uppercase text-muted-foreground">{props.label}</dt>
      <dd className="mt-1 break-words font-mono text-xs">{props.value}</dd>
    </div>
  );
}
