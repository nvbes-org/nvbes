import { ClipboardButton } from '@nvbes/web-ui';
import { Link2, ShieldOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { DriveEmptyState } from './DriveViewState';
import { revokeShareLink } from './drive.workspace.store';
import type {
  DriveEntry,
  DriveShareLink,
  DriveShareStatus,
  DriveWorkspaceState,
} from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveSharedLinksView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  if (state.shareLinks.length === 0) {
    return (
      <DriveEmptyState
        title="Aucun lien partage"
        description="Creez un lien depuis un fichier pour donner un acces externe controle."
      />
    );
  }

  function handleRevoke(linkId: string) {
    onStateChange(revokeShareLink(state, linkId));
  }

  return (
    <section className="grid gap-3">
      {state.shareLinks.map((link) => {
        const entry = entryForLink(state.entries, link);
        const isRevoked = link.status === 'revoked';

        return (
          <Card key={link.id} className="p-4 transition hover:shadow-md">
            <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <Link2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
                  <h3 className="truncate font-medium">{entry?.name ?? link.label}</h3>
                </div>
                <p className="mt-1 truncate text-sm text-muted-foreground">{link.label}</p>
              </div>

              <dl className="grid gap-3 text-sm sm:grid-cols-3 lg:min-w-[420px]">
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                    Expiration
                  </dt>
                  <dd className="font-medium">{formatNullableDate(link.expiresAt)}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">Acces</dt>
                  <dd className="font-medium">{link.accessCount} acces</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-wide text-muted-foreground">Statut</dt>
                  <dd className="font-medium">{shareStatusLabel(link.status)}</dd>
                </div>
              </dl>

              <div className="flex flex-wrap gap-2">
                <ClipboardButton
                  value={shareLinkUrl(link)}
                  label={`Copier le lien ${entry?.name ?? link.label}`}
                  className="h-9"
                />
                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  disabled={isRevoked}
                  onClick={() => handleRevoke(link.id)}
                  aria-label={`Revoquer le lien ${entry?.name ?? link.label}`}
                >
                  <ShieldOff data-icon="inline-start" aria-hidden="true" />
                  {isRevoked ? 'Revoque' : 'Revoquer'}
                </Button>
              </div>
            </div>
          </Card>
        );
      })}
    </section>
  );
}

function entryForLink(entries: DriveEntry[], link: DriveShareLink): DriveEntry | undefined {
  return entries.find((entry) => entry.id === link.entryId);
}

function shareLinkUrl(link: DriveShareLink): string {
  return `${window.location.origin}/share/${link.token}`;
}

function formatNullableDate(value: string | null): string {
  return value ? DATE_FORMATTER.format(new Date(value)) : 'Sans expiration';
}

function shareStatusLabel(status: DriveShareStatus): string {
  if (status === 'shared') return 'Actif';
  if (status === 'revoked') return 'Revoque';
  if (status === 'expired') return 'Expire';
  return 'Prive';
}
