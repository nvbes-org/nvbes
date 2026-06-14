import { useMutation } from '@tanstack/react-query';
import { AlertTriangle, Bug } from 'lucide-react';
import { useState } from 'react';
import { debugDeveloperToken } from '../developer.api';
import { summarizeAccessDecision } from './TokenDebuggerPage.helpers';

export function TokenDebuggerPage() {
  const [accessToken, setAccessToken] = useState('');
  const debugMutation = useMutation({ mutationFn: debugDeveloperToken });
  const result = debugMutation.data ?? null;

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Bug className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Token debugger</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Inspect access token claims, scopes, audience, expiration, and access decision.
          </p>
        </div>
      </div>
      <form
        className="rounded-lg border border-border bg-card p-5"
        onSubmit={(event) => {
          event.preventDefault();
          debugMutation.mutate(accessToken);
        }}
      >
        <label className="grid gap-2 text-sm font-medium">
          Access token
          <textarea
            className="min-h-32 rounded-md border border-input bg-background px-3 py-2 font-mono text-xs"
            value={accessToken}
            onChange={(event) => setAccessToken(event.currentTarget.value)}
          />
        </label>
        <button
          className="mt-4 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
          disabled={debugMutation.isPending || accessToken.trim().length === 0}
          type="submit"
        >
          Inspect token
        </button>
        {debugMutation.isError ? (
          <div className="mt-4 flex items-center gap-2 text-sm text-red-600">
            <AlertTriangle className="h-4 w-4" />
            Token debugger unavailable
          </div>
        ) : null}
      </form>
      <article className="rounded-lg border border-border bg-card p-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h3 className="text-base font-semibold">{summarizeAccessDecision(result)}</h3>
          {result ? (
            <span className="rounded-md border border-border px-2 py-1 font-mono text-xs">
              {result.token_hash_prefix}
            </span>
          ) : null}
        </div>
        {result?.claims ? (
          <dl className="mt-4 grid gap-3 text-sm md:grid-cols-2">
            <Field label="Subject" value={result.claims.subject} />
            <Field label="Audience" value={result.claims.audience} />
            <Field label="Client" value={result.claims.client_id ?? '-'} />
            <Field label="Tenant" value={result.claims.tenant_id ?? '-'} />
            <Field label="Expires" value={result.claims.expires_at} />
            <Field label="Scopes" value={result.claims.scopes.join(' ') || '-'} />
          </dl>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">No decoded claims.</p>
        )}
      </article>
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
