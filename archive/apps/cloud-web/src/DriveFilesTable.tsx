import {
  Archive,
  FileText,
  FileType2,
  Folder,
  Image,
  MoreHorizontal,
  Table2,
  Video,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
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
    <Card className="overflow-hidden py-0">
      <Table className="min-w-[760px]">
        <TableHeader className="bg-muted/40 text-xs uppercase tracking-wide text-muted-foreground">
          <TableRow>
            <TableHead className="w-10" aria-label="Selection" />
            <TableHead>Nom</TableHead>
            <TableHead>Statut</TableHead>
            <TableHead>Proprietaire</TableHead>
            <TableHead>Taille</TableHead>
            <TableHead>Modifie le</TableHead>
            <TableHead className="w-24 text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map((entry) => {
            const Icon = iconForKind(entry.kind);
            const isSelected = selectedEntryIds.includes(entry.id);

            return (
              <TableRow
                key={entry.id}
                data-state={isSelected ? 'selected' : undefined}
                onContextMenu={(event) => {
                  event.preventDefault();
                  onSecondarySelect(entry.id);
                }}
              >
                <TableCell>
                  <Checkbox
                    checked={isSelected}
                    onCheckedChange={() => onToggleSelection(entry.id)}
                    aria-label={`Selectionner ${entry.name}`}
                  />
                </TableCell>
                <TableCell>
                  <Button
                    type="button"
                    variant="ghost"
                    className="h-auto min-w-0 justify-start px-0 font-medium hover:bg-transparent"
                    onClick={() => onOpenDetails(entry.id)}
                  >
                    <Icon data-icon="inline-start" className="text-primary" aria-hidden="true" />
                    <span className="truncate">{entry.name}</span>
                  </Button>
                </TableCell>
                <TableCell className="text-muted-foreground">{statusLabel(entry)}</TableCell>
                <TableCell className="text-muted-foreground">
                  {ownerName(entry.ownerId, members)}
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {formatBytes(entry.sizeBytes)}
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {formatDate(entry.updatedAt)}
                </TableCell>
                <TableCell className="text-right">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => onOpenDetails(entry.id)}
                  >
                    Details
                    <MoreHorizontal data-icon="inline-end" aria-hidden="true" />
                  </Button>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </Card>
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
