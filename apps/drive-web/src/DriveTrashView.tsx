import { RotateCcw, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DriveEmptyState } from './DriveViewState';
import { restoreTrashEntry } from './drive.workspace.store';
import type { DriveEntry, DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveTrashView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const trashedEntries = state.entries.filter((entry) => entry.status === 'trashed');

  if (trashedEntries.length === 0) {
    return (
      <DriveEmptyState
        title="Corbeille vide"
        description="Les fichiers supprimes apparaitront ici avant restauration ou purge definitive."
      />
    );
  }

  function handleRestore(entryId: string) {
    onStateChange(restoreTrashEntry(state, entryId));
  }

  return (
    <section className="grid gap-3">
      {trashedEntries.map((entry) => (
        <article
          key={entry.id}
          className="flex flex-col gap-4 rounded-2xl border border-border/70 bg-background p-4 shadow-sm sm:flex-row sm:items-center sm:justify-between"
        >
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <Trash2 className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
              <h3 className="truncate font-medium">{entry.name}</h3>
            </div>
            <p className="mt-1 text-sm text-muted-foreground">Supprime le {formatDate(entry.updatedAt)}</p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => handleRestore(entry.id)}
            aria-label={`Restaurer ${entry.name}`}
          >
            <RotateCcw className="size-4" aria-hidden="true" />
            Restaurer
          </Button>
        </article>
      ))}
    </section>
  );
}

function formatDate(value: DriveEntry['updatedAt']): string {
  return DATE_FORMATTER.format(new Date(value));
}
