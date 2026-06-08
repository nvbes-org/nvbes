import { Archive, FileText, FileType2, Folder, Image, Table2, Video } from 'lucide-react';
import { cn } from './lib/classnames';
import type { DriveEntry, DriveEntryKind, DriveMember } from './drive.workspace.types';

export function DriveFilesGrid({
  entries,
  members,
  selectedEntryIds,
  onOpenDetails,
  onToggleSelection,
}: {
  entries: DriveEntry[];
  members: DriveMember[];
  selectedEntryIds: string[];
  onOpenDetails: (entryId: string) => void;
  onToggleSelection: (entryId: string) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {entries.map((entry) => {
        const Icon = iconForKind(entry.kind);
        const isSelected = selectedEntryIds.includes(entry.id);

        return (
          <article
            key={entry.id}
            className={cn(
              'group/file rounded-2xl border border-border/70 bg-background p-4 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md',
              isSelected && 'border-primary/40 bg-primary/5',
            )}
          >
            <div className="flex items-start justify-between gap-3">
              <button
                type="button"
                className="flex min-w-0 flex-1 items-start gap-3 text-left"
                onClick={() => onOpenDetails(entry.id)}
              >
                <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-muted text-primary">
                  <Icon className="size-5" aria-hidden="true" />
                </span>
                <span className="min-w-0">
                  <span className="block truncate font-medium">{entry.name}</span>
                  <span className="block truncate text-sm text-muted-foreground">
                    {ownerName(entry.ownerId, members)}
                  </span>
                </span>
              </button>
              <input
                type="checkbox"
                checked={isSelected}
                onChange={() => onToggleSelection(entry.id)}
                aria-label={`Selectionner ${entry.name}`}
                className="mt-1 size-4 rounded border-border"
              />
            </div>
          </article>
        );
      })}
    </div>
  );
}

function iconForKind(kind: DriveEntryKind) {
  if (kind === 'folder') return Folder;
  if (kind === 'image') return Image;
  if (kind === 'video') return Video;
  if (kind === 'archive') return Archive;
  if (kind === 'spreadsheet') return Table2;
  if (kind === 'document') return FileText;
  return FileType2;
}

function ownerName(ownerId: string, members: DriveMember[]): string {
  return members.find((member) => member.id === ownerId)?.name ?? 'Inconnu';
}
