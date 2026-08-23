import { Archive, FileText, FileType2, Folder, Image, Table2, Video } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { cn } from './lib/classnames';
import type { DriveEntry, DriveEntryKind, DriveMember } from './drive.workspace.types';

const BYTE_FORMATTER = new Intl.NumberFormat('fr-FR', {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

export function DriveFilesGrid({
  entries,
  members,
  selectedEntryIds,
  onOpenDetails,
  onSecondarySelect,
  onToggleSelection,
}: {
  entries: DriveEntry[];
  members: DriveMember[];
  selectedEntryIds: string[];
  onOpenDetails: (entryId: string) => void;
  onSecondarySelect: (entryId: string) => void;
  onToggleSelection: (entryId: string) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
      {entries.map((entry) => {
        const Icon = iconForKind(entry.kind);
        const isSelected = selectedEntryIds.includes(entry.id);

        return (
          <Card
            key={entry.id}
            className={cn(
              'group/file overflow-hidden py-0 transition hover:-translate-y-0.5 hover:shadow-md',
              isSelected && 'border-primary/40 bg-primary/5',
            )}
            onContextMenu={(event) => {
              event.preventDefault();
              onSecondarySelect(entry.id);
            }}
          >
            <div
              className={cn(
                'relative aspect-[4/3] w-full overflow-hidden',
                previewTone(entry.kind),
              )}
            >
              <span className="absolute inset-0 bg-linear-to-br from-white/18 via-white/0 to-black/10" />
              <span className="absolute inset-x-4 top-4 flex items-start justify-between gap-3">
                <Badge
                  variant="outline"
                  className="border-white/35 bg-white/18 text-[11px] uppercase tracking-[0.18em] text-white/90 backdrop-blur-sm"
                >
                  {kindLabel(entry.kind)}
                </Badge>
                <Checkbox
                  checked={isSelected}
                  onClick={(event) => event.stopPropagation()}
                  onCheckedChange={() => onToggleSelection(entry.id)}
                  aria-label={`Selectionner ${entry.name}`}
                  className="mt-0.5 border-white/70 bg-white/90"
                />
              </span>
              <Button
                type="button"
                variant="ghost"
                className="absolute inset-0 h-auto w-auto rounded-none p-0 opacity-0 hover:bg-transparent"
                aria-label={`Ouvrir ${entry.name}`}
                onClick={() => onOpenDetails(entry.id)}
              />
              <span className="absolute inset-0 grid place-items-center">
                <span className="grid size-20 place-items-center rounded-[1.75rem] border border-white/20 bg-white/12 text-white shadow-lg shadow-black/10 backdrop-blur-sm transition duration-200 group-hover/file:scale-105">
                  <Icon className="size-9" aria-hidden="true" />
                </span>
              </span>
            </div>
            <div className="flex items-start justify-between gap-3 p-4">
              <Button
                type="button"
                variant="ghost"
                className="h-auto min-w-0 flex-1 justify-start p-0 text-left hover:bg-transparent"
                onClick={() => onOpenDetails(entry.id)}
              >
                <span className="block truncate text-base font-medium">{entry.name}</span>
                <span className="mt-1 block truncate text-sm text-muted-foreground">
                  {ownerName(entry.ownerId, members)}
                </span>
                <span className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                  <span>{formatBytes(entry.sizeBytes)}</span>
                  <span>{statusLabel(entry)}</span>
                </span>
              </Button>
            </div>
          </Card>
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

function kindLabel(kind: DriveEntryKind): string {
  if (kind === 'folder') return 'Dossier';
  if (kind === 'image') return 'Image';
  if (kind === 'video') return 'Video';
  if (kind === 'archive') return 'Archive';
  if (kind === 'spreadsheet') return 'Tableur';
  if (kind === 'document') return 'Document';
  return 'Fichier';
}

function previewTone(kind: DriveEntryKind): string {
  if (kind === 'folder') return 'bg-linear-to-br from-amber-400 via-orange-500 to-amber-700';
  if (kind === 'image') return 'bg-linear-to-br from-sky-400 via-cyan-500 to-blue-700';
  if (kind === 'video') return 'bg-linear-to-br from-fuchsia-500 via-rose-500 to-orange-500';
  if (kind === 'archive') return 'bg-linear-to-br from-slate-500 via-slate-700 to-slate-900';
  if (kind === 'spreadsheet')
    return 'bg-linear-to-br from-emerald-400 via-green-600 to-emerald-800';
  if (kind === 'document') return 'bg-linear-to-br from-indigo-400 via-violet-600 to-indigo-800';
  return 'bg-linear-to-br from-zinc-400 via-zinc-600 to-zinc-800';
}

function ownerName(ownerId: string, members: DriveMember[]): string {
  return members.find((member) => member.id === ownerId)?.name ?? 'Inconnu';
}

function statusLabel(entry: DriveEntry): string {
  if (entry.shareStatus === 'shared') return `Partage avec ${entry.sharedWithCount}`;
  if (entry.starred) return 'Favori';
  return 'Prive';
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '-';

  const units = ['o', 'Ko', 'Mo', 'Go', 'To'];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unitIndex;

  return `${BYTE_FORMATTER.format(value)} ${units[unitIndex]}`;
}
