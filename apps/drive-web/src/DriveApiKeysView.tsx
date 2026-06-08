import { KeyRound, ShieldOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { revokeApiKey } from './drive.workspace.store';
import type { DriveApiKey, DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveApiKeysView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  function handleRevoke(keyId: string) {
    onStateChange(revokeApiKey(state, keyId));
  }

  return (
    <section className="grid gap-4">
      <div className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
          Integrations
        </p>
        <h2 className="mt-1 text-lg font-semibold">Cles API Drive</h2>
        <p className="text-sm text-muted-foreground">
          Les nouvelles cles seront creees depuis le backend public API. Cette vue permet deja la
          revocation locale des cles exposees.
        </p>
      </div>

      <div className="grid gap-3">
        {state.apiKeys.map((apiKey) => {
          const isRevoked = apiKey.status === 'revoked';

          return (
            <article
              key={apiKey.id}
              className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm"
            >
              <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex min-w-0 items-start gap-3">
                  <span className="grid size-10 shrink-0 place-items-center rounded-2xl bg-muted text-primary">
                    <KeyRound className="size-5" aria-hidden="true" />
                  </span>
                  <div className="min-w-0">
                    <h3 className="truncate font-medium">{apiKey.label}</h3>
                    <p className="font-mono text-sm text-muted-foreground">{apiKey.prefix}...</p>
                  </div>
                </div>

                <dl className="grid gap-3 text-sm sm:grid-cols-3 lg:min-w-[500px]">
                  <div>
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">Scopes</dt>
                    <dd className="font-medium">{scopesForKey(apiKey).join(', ')}</dd>
                  </div>
                  <div>
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">Statut</dt>
                    <dd className="font-medium">{apiKeyStatusLabel(apiKey.status)}</dd>
                  </div>
                  <div>
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">Dernier usage</dt>
                    <dd className="font-medium">{formatNullableDate(apiKey.lastUsedAt)}</dd>
                  </div>
                </dl>

                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  disabled={isRevoked}
                  onClick={() => handleRevoke(apiKey.id)}
                  aria-label={`Revoquer la cle ${apiKey.label}`}
                >
                  <ShieldOff className="size-4" aria-hidden="true" />
                  {isRevoked ? 'Deja revoquee' : 'Revoquer'}
                </Button>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function scopesForKey(apiKey: DriveApiKey): string[] {
  if (apiKey.status === 'revoked') {
    return ['aucun'];
  }

  return ['files:read', 'files:write'];
}

function apiKeyStatusLabel(status: DriveApiKey['status']): string {
  if (status === 'active') return 'Active';
  return 'Revoquee';
}

function formatNullableDate(value: string | null): string {
  return value ? DATE_FORMATTER.format(new Date(value)) : 'Jamais';
}
