import { FileText, FolderKanban, Users } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { DriveEmptyState } from './DriveViewState';
import type { DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveSharedWithMeView({
  state,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const sharedEntries = state.entries.filter(
    (entry) => entry.status === 'active' && entry.ownerId !== 'member-owner',
  );

  if (sharedEntries.length === 0) {
    return (
      <DriveEmptyState
        title="Aucun element partage"
        description="Les fichiers et dossiers partages par les membres de votre espace apparaitront ici."
      />
    );
  }

  return (
    <section className="grid gap-3">
      {sharedEntries.map((entry) => {
        const owner = state.members.find((member) => member.id === entry.ownerId);

        return (
          <Card
            key={entry.id}
            className="flex flex-col gap-4 p-4 sm:flex-row sm:items-center sm:justify-between"
          >
            <div className="flex min-w-0 items-center gap-3">
              <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary">
                {entry.kind === 'folder' ? (
                  <FolderKanban className="size-4" aria-hidden="true" />
                ) : (
                  <FileText className="size-4" aria-hidden="true" />
                )}
              </div>
              <div className="min-w-0">
                <h3 className="truncate font-medium">{entry.name}</h3>
                <p className="mt-0.5 flex items-center gap-1.5 text-sm text-muted-foreground">
                  <Users className="size-3.5 shrink-0" aria-hidden="true" />
                  <span className="truncate">
                    {owner?.name ?? 'Membre inconnu'} &middot; Modifie le{' '}
                    {DATE_FORMATTER.format(new Date(entry.updatedAt))}
                  </span>
                </p>
              </div>
            </div>
          </Card>
        );
      })}
    </section>
  );
}
