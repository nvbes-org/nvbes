import { AlertTriangle, ShieldCheck } from 'lucide-react';
import type { DriveSecurityEvent, DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveSecurityView({ state }: { state: DriveWorkspaceState }) {
  const activeSharedLinksCount = state.shareLinks.filter((link) => link.status === 'shared').length;

  return (
    <section className="grid gap-4">
      <div className="grid gap-3 md:grid-cols-2">
        <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
          <div className="flex items-start gap-3">
            <span className="grid size-10 shrink-0 place-items-center rounded-2xl bg-primary/10 text-primary">
              <ShieldCheck className="size-5" aria-hidden="true" />
            </span>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Posture
              </p>
              <h2 className="mt-1 text-lg font-semibold">Liens actifs controles</h2>
              <p className="mt-1 text-sm text-muted-foreground">
                {activeSharedLinksCount} lien{activeSharedLinksCount > 1 ? 's' : ''} partage
                {activeSharedLinksCount > 1 ? 's' : ''} actif{activeSharedLinksCount > 1 ? 's' : ''}.
              </p>
            </div>
          </div>
        </article>

        <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
          <div className="flex items-start gap-3">
            <span className="grid size-10 shrink-0 place-items-center rounded-2xl bg-muted text-muted-foreground">
              <AlertTriangle className="size-5" aria-hidden="true" />
            </span>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Expiration
              </p>
              <h2 className="mt-1 text-lg font-semibold">Principe limite par defaut</h2>
              <p className="mt-1 text-sm text-muted-foreground">
                Les liens partages doivent porter une date d'expiration; les liens sans expiration
                sont a auditer avant ouverture externe.
              </p>
            </div>
          </div>
        </article>
      </div>

      <div className="grid gap-3">
        {state.securityEvents.map((event) => (
          <article
            key={event.id}
            className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm"
          >
            <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className={severityClassName(event.severity)}>{severityLabel(event.severity)}</span>
                  <h3 className="truncate font-medium">{event.action}</h3>
                </div>
                <p className="mt-1 truncate text-sm text-muted-foreground">{event.target}</p>
              </div>
              <dl className="grid gap-3 text-sm sm:grid-cols-3 lg:min-w-[460px]">
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">Acteur</dt>
                  <dd className="font-medium">{actorName(state, event)}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">Date</dt>
                  <dd className="font-medium">{formatDate(event.createdAt)}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">Severite</dt>
                  <dd className="font-medium">{severityLabel(event.severity)}</dd>
                </div>
              </dl>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function actorName(state: DriveWorkspaceState, event: DriveSecurityEvent): string {
  return state.members.find((member) => member.id === event.actorId)?.name ?? 'Acteur inconnu';
}

function formatDate(value: string): string {
  return DATE_FORMATTER.format(new Date(value));
}

function severityLabel(severity: DriveSecurityEvent['severity']): string {
  if (severity === 'critical') return 'Critique';
  if (severity === 'warning') return 'Attention';
  return 'Info';
}

function severityClassName(severity: DriveSecurityEvent['severity']): string {
  const base = 'rounded-full px-2 py-0.5 text-xs font-medium';
  if (severity === 'critical') return `${base} bg-destructive/10 text-destructive`;
  if (severity === 'warning') return `${base} bg-amber-500/10 text-amber-700`;
  return `${base} bg-primary/10 text-primary`;
}
