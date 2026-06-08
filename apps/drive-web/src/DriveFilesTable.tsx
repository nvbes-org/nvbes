import { Archive, FileText, FileType2, Folder, Image, MoreHorizontal, Table2, Video } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from './lib/classnames';
import type { DriveEntry, DriveEntryKind, DriveMember } from './drive.workspace.types';

const BYTE_FORMATTER = new Intl.NumberFormat('fr-FR', {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveFilesTable({
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
    <div className="overflow-hidden rounded-2xl border border-border/70 bg-background shadow-sm">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[760px] text-sm">
          <thead className="border-b border-border/70 bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr>
              <th className="w-10 px-3 py-2" aria-label="Selection" />
              <th className="px-3 py-2 font-medium">Nom</th>
              <th className="px-3 py-2 font-medium">Statut</th>
              <th className="px-3 py-2 font-medium">Proprietaire</th>
              <th className="px-3 py-2 font-medium">Taille</th>
              <th className="px-3 py-2 font-medium">Modifie le</th>
              <th className="w-24 px-3 py-2 text-right font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => {
              const Icon = iconForKind(entry.kind);
              const isSelected = selectedEntryIds.includes(entry.id);

              return (
                <tr
                  key={entry.id}
                  className={cn('border-b border-border/50 last:border-0', isSelected && 'bg-muted/45')}
                >
                  <td className="px-3 py-2">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => onToggleSelection(entry.id)}
                      aria-label={`Selectionner ${entry.name}`}
                      className="size-4 rounded border-border"
                    />
                  </td>
                  <td className="px-3 py-2">
                    <button
                      type="button"
                      className="flex min-w-0 items-center gap-2 text-left font-medium hover:text-primary"
                      onClick={() => onOpenDetails(entry.id)}
                    >
                      <Icon className="size-4 shrink-0 text-primary" aria-hidden="true" />
                      <span className="truncate">{entry.name}</span>
                    </button>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{statusLabel(entry)}</td>
                  <td className="px-3 py-2 text-muted-foreground">{ownerName(entry.ownerId, members)}</td>
                  <td className="px-3 py-2 text-muted-foreground">{formatBytes(entry.sizeBytes)}</td>
                  <td className="px-3 py-2 text-muted-foreground">{formatDate(entry.updatedAt)}</td>
                  <td className="px-3 py-2 text-right">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => onOpenDetails(entry.id)}
                    >
                      Details
                      <MoreHorizontal className="size-4" aria-hidden="true" />
                    </Button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
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

function statusLabel(entry: DriveEntry): string {
  if (entry.shareStatus === 'shared') return `Partage avec ${entry.sharedWithCount}`;
  if (entry.starred) return 'Favori';
  return 'Prive';
}

function ownerName(ownerId: string, members: DriveMember[]): string {
  return members.find((member) => member.id === ownerId)?.name ?? 'Inconnu';
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '-';

  const units = ['o', 'Ko', 'Mo', 'Go', 'To'];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unitIndex;

  return `${BYTE_FORMATTER.format(value)} ${units[unitIndex]}`;
}

function formatDate(value: string): string {
  return DATE_FORMATTER.format(new Date(value));
}
