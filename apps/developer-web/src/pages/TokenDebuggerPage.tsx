import { useMutation } from '@tanstack/react-query';
import { AlertTriangle, Bug, CheckCircle2, Clock3, ShieldAlert, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { debugDeveloperToken } from '../developer.api';
import type { DebugDeveloperToken } from '../developer.schemas';
import {
  accessDecisionTone,
  describeAccessDecision,
  formatTokenTimestamp,
  summarizeAccessDecision,
  summarizeExpiration,
} from './TokenDebuggerPage.helpers';

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
        <DecisionHeader result={result} />
        {result?.claims ? (
          <div className="mt-5 space-y-5">
            <section className="grid gap-3 md:grid-cols-3">
              <Signal
                label="Active"
                value={result.active ? 'Yes' : 'No'}
                tone={result.active ? 'success' : 'warning'}
              />
              <Signal label="Token type" value={result.claims.token_type} tone="muted" />
              <Signal
                label="Expiration"
                value={summarizeExpiration(result.claims.expires_at)}
                tone={result.active ? 'success' : 'warning'}
              />
            </section>
            <dl className="grid gap-3 text-sm md:grid-cols-2">
              <Field label="Subject" value={result.claims.subject} />
              <Field label="Audience" value={result.claims.audience} />
              <Field label="Issuer" value={result.claims.issuer} />
              <Field label="Client" value={result.claims.client_id ?? '-'} />
              <Field label="Tenant" value={result.claims.tenant_id ?? '-'} />
              <Field label="Workspace" value={result.claims.workspace_id ?? '-'} />
              <Field label="Issued at" value={formatTokenTimestamp(result.claims.issued_at)} />
              <Field label="Not before" value={formatTokenTimestamp(result.claims.not_before)} />
              <Field label="Expires at" value={formatTokenTimestamp(result.claims.expires_at)} />
              <Field label="ACR" value={result.claims.acr ?? '-'} />
              <Field label="AMR" value={result.claims.amr.join(' ') || '-'} />
            </dl>
            <Scopes scopes={result.claims.scopes} />
            <ClaimsJson result={result} />
          </div>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">No decoded claims.</p>
        )}
      </article>
    </section>
  );
}

function DecisionHeader({ result }: { result: DebugDeveloperToken | null }) {
  const tone = accessDecisionTone(result);
  const Icon =
    tone === 'success'
      ? ShieldCheck
      : tone === 'warning'
        ? ShieldAlert
        : tone === 'danger'
          ? AlertTriangle
          : Bug;

  return (
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div className="flex min-w-0 gap-3">
        <div className={`mt-0.5 rounded-md border p-2 ${toneClass(tone)}`}>
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0">
          <h3 className="text-base font-semibold">{summarizeAccessDecision(result)}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{describeAccessDecision(result)}</p>
        </div>
      </div>
      {result ? (
        <span className="rounded-md border border-border px-2 py-1 font-mono text-xs">
          sha256:{result.token_hash_prefix}
        </span>
      ) : null}
    </div>
  );
}

function Signal(props: { label: string; value: string; tone: 'success' | 'warning' | 'muted' }) {
  const Icon = props.tone === 'success' ? CheckCircle2 : props.tone === 'warning' ? Clock3 : Bug;

  return (
    <div className={`rounded-md border p-3 ${toneClass(props.tone)}`}>
      <div className="flex items-center gap-2 text-xs font-medium uppercase">
        <Icon className="h-3.5 w-3.5" />
        {props.label}
      </div>
      <p className="mt-2 break-words text-sm font-semibold">{props.value}</p>
    </div>
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

function Scopes({ scopes }: { scopes: string[] }) {
  return (
    <section>
      <h4 className="text-xs font-medium uppercase text-muted-foreground">Scopes</h4>
      {scopes.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-2">
          {scopes.map((scope) => (
            <span
              key={scope}
              className="rounded-md border border-border bg-background px-2 py-1 font-mono text-xs"
            >
              {scope}
            </span>
          ))}
        </div>
      ) : (
        <p className="mt-2 text-sm text-muted-foreground">No scopes in token.</p>
      )}
    </section>
  );
}

function ClaimsJson({ result }: { result: DebugDeveloperToken }) {
  return (
    <details className="rounded-md border border-border bg-background p-3">
      <summary className="cursor-pointer text-sm font-medium">Decoded claims JSON</summary>
      <pre className="mt-3 max-h-80 overflow-auto text-xs">
        {JSON.stringify(result.claims, null, 2)}
      </pre>
    </details>
  );
}

function toneClass(tone: 'success' | 'warning' | 'danger' | 'muted') {
  switch (tone) {
    case 'success':
      return 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700';
    case 'warning':
      return 'border-amber-500/25 bg-amber-500/10 text-amber-700';
    case 'danger':
      return 'border-red-500/25 bg-red-500/10 text-red-700';
    case 'muted':
      return 'border-border bg-background text-muted-foreground';
  }
}
