import { Star } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { DriveEmptyState } from './DriveViewState';
import type { DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveStarredView({
  state,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const starredEntries = state.entries.filter(
    (entry) => entry.status === 'active' && entry.starred,
  );

  if (starredEntries.length === 0) {
    return (
      <DriveEmptyState
        title="Aucun element suivi"
        description="Marquez des fichiers ou dossiers d'une etoile pour les retrouver rapidement ici."
      />
    );
  }

  return (
    <section className="grid gap-3">
      {starredEntries.map((entry) => {
        const owner = state.members.find((member) => member.id === entry.ownerId);

        return (
          <Card
            key={entry.id}
            className="flex flex-col gap-4 p-4 sm:flex-row sm:items-center sm:justify-between"
          >
            <div className="flex min-w-0 items-center gap-3">
              <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-amber-500/10 text-amber-500">
                <Star className="size-4" aria-hidden="true" />
              </div>
              <div className="min-w-0">
                <h3 className="truncate font-medium">{entry.name}</h3>
                <p className="mt-0.5 text-sm text-muted-foreground">
                  {entry.kind === 'folder' ? 'Dossier' : 'Fichier'} &middot;{' '}
                  {owner?.name ?? 'Membre inconnu'} &middot;{' '}
                  {DATE_FORMATTER.format(new Date(entry.updatedAt))}
                </p>
              </div>
            </div>
          </Card>
        );
      })}
    </section>
  );
}
