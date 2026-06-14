import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import { inspectDeveloperToken } from '@/developer.api';
import { Field, PageHeader, buttonClass, formString, textareaClass } from './Portal.shared';

function decodeJwtSegment(segment: string): unknown {
  const normalized = segment.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');

  return JSON.parse(window.atob(padded)) as unknown;
}

export function PortalTokenInspectorPage() {
  const [parsed, setParsed] = useState<unknown>(undefined);
  const [parseError, setParseError] = useState<string | null>(null);
  const inspectToken = useMutation({ mutationFn: inspectDeveloperToken });

  return (
    <section>
      <PageHeader
        title="Token inspector"
        body="Parse JWT headers and claims locally, then ask the backend for tenant-scoped validity."
      />
      <form
        className="grid gap-4 rounded-md border border-border bg-card p-4"
        onSubmit={(event) => {
          event.preventDefault();
          const token = formString(new FormData(event.currentTarget), 'token').trim();
          if (!token) {
            setParseError('Token is required.');
            return;
          }
          const [, payload] = token.split('.');
          try {
            setParsed(payload ? decodeJwtSegment(payload) : undefined);
            setParseError(null);
          } catch {
            setParsed(undefined);
            setParseError('Token payload is not valid base64url JSON.');
          }
          inspectToken.mutate(token);
        }}
      >
        <Field label="Access token">
          <textarea name="token" className={textareaClass} />
        </Field>
        {parseError ? <p className="text-sm text-destructive">{parseError}</p> : null}
        <button type="submit" className={buttonClass} disabled={inspectToken.isPending}>
          Inspect token
        </button>
      </form>
      <div className="mt-6 grid gap-4 md:grid-cols-2">
        <pre className="overflow-auto rounded-md border border-border bg-card p-4 text-xs">
          {parsed === undefined ? 'No parsed claims yet.' : JSON.stringify(parsed, null, 2)}
        </pre>
        <pre className="overflow-auto rounded-md border border-border bg-card p-4 text-xs">
          {inspectToken.data ? JSON.stringify(inspectToken.data, null, 2) : 'No backend result yet.'}
        </pre>
      </div>
    </section>
  );
}
