import { AlertTriangle, RotateCcwKey } from 'lucide-react';
import { useState } from 'react';
import { buildSecretRotationPayload } from './SecretsPage.helpers';

export function SecretsPage() {
  const [overlapHours, setOverlapHours] = useState(24);
  const [error, setError] = useState<string | null>(null);
  const payload = buildPayloadPreview(overlapHours);

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <RotateCcwKey className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Secret rotation</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Prepare a client secret rotation with an explicit overlap window.
          </p>
        </div>
      </div>
      <form
        className="rounded-lg border border-border bg-card p-5"
        onSubmit={(event) => {
          event.preventDefault();
          try {
            buildSecretRotationPayload({ overlapHours });
            setError(null);
          } catch (cause) {
            setError(cause instanceof Error ? cause.message : 'Invalid rotation payload');
          }
        }}
      >
        <label className="grid gap-2 text-sm font-medium">
          Overlap window
          <input
            className="h-10 rounded-md border border-input bg-background px-3 text-sm"
            min={1}
            type="number"
            value={overlapHours}
            onChange={(event) => setOverlapHours(Number(event.currentTarget.value))}
          />
        </label>
        <button
          className="mt-4 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          type="submit"
        >
          Validate rotation
        </button>
        {error ? (
          <div className="mt-4 flex items-center gap-2 text-sm text-red-600">
            <AlertTriangle className="h-4 w-4" />
            {error}
          </div>
        ) : null}
        <pre className="mt-4 overflow-x-auto rounded-md bg-muted p-3 text-xs">
          {JSON.stringify(payload, null, 2)}
        </pre>
      </form>
    </section>
  );
}

function buildPayloadPreview(overlapHours: number) {
  try {
    return buildSecretRotationPayload({ overlapHours });
  } catch {
    return { overlap_hours: null };
  }
}
