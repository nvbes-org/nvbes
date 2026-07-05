import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Boxes, RefreshCw, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import {
  getDeveloperSandbox,
  resetDeveloperSandbox,
  upsertDeveloperSandbox,
} from '../developer.api';
import type { DeveloperSandboxDataProfile } from '../developer.schemas';
import { PageHeader, buttonClass, inputClass } from './Portal.shared';

const dataProfileOptions: { value: DeveloperSandboxDataProfile; label: string }[] = [
  { value: 'minimal', label: 'Minimal' },
  { value: 'oauth', label: 'OAuth flows' },
  { value: 'full', label: 'Full integration' },
];

export function SandboxPage() {
  const queryClient = useQueryClient();
  const [dataProfile, setDataProfile] = useState<DeveloperSandboxDataProfile>('minimal');
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
      <PageHeader
        title="Sandbox tenant"
        body="Dedicated test environment per client tenant, with isolated identity data and integration fixtures."
      />
      <div className="grid gap-4 lg:grid-cols-[360px_1fr]">
        <form
          className="rounded-md border border-border bg-card p-5"
          onSubmit={(event) => {
            event.preventDefault();
            upsertMutation.mutate(dataProfile);
          }}
        >
          <label className="grid gap-2 text-sm font-medium">
            Data profile
            <select
              className={inputClass}
              value={dataProfile}
              onChange={(event) =>
                setDataProfile(event.currentTarget.value as DeveloperSandboxDataProfile)
              }
            >
              {dataProfileOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <div className="mt-4 flex flex-wrap gap-2">
            <button className={buttonClass} disabled={upsertMutation.isPending} type="submit">
              <Boxes className="mr-2 h-4 w-4" />
              {sandbox ? 'Update sandbox' : 'Create sandbox'}
            </button>
            <button
              className="inline-flex h-10 items-center justify-center rounded-md border border-border px-4 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
              disabled={!sandbox || resetMutation.isPending}
              type="button"
              onClick={() => resetMutation.mutate()}
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              Reset data
            </button>
          </div>
        </form>
        <article className="rounded-md border border-border bg-card p-5">
          {sandbox ? (
            <dl className="grid gap-3 text-sm md:grid-cols-2">
              <Field label="Client tenant" value={sandbox.tenant_id} />
              <Field label="Sandbox tenant" value={sandbox.sandbox_tenant_id} />
              <Field label="Name" value={sandbox.sandbox_name} />
              <Field label="Slug" value={sandbox.sandbox_slug} />
              <Field label="Status" value={sandbox.status} />
              <Field label="Data profile" value={sandbox.data_profile} />
              <Field label="Updated" value={sandbox.updated_at} />
            </dl>
          ) : (
            <div className="flex items-start gap-3">
              <ShieldCheck className="mt-0.5 h-5 w-5 text-primary" />
              <p className="text-sm text-muted-foreground">
                No sandbox tenant exists for this client tenant.
              </p>
            </div>
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
