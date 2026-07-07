import { FolderPlus, Grid2X2, List, Upload } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Label } from '@/components/ui/label';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import type { DriveSortKey, DriveViewMode } from './drive.workspace.types';

const SORT_OPTIONS: Array<{ value: DriveSortKey; label: string }> = [
  { value: 'updated', label: 'Modifie recemment' },
  { value: 'name', label: 'Nom' },
  { value: 'size', label: 'Taille' },
  { value: 'kind', label: 'Type' },
];

export function DriveFilesToolbar({
  selectedCount,
  viewMode,
  sort,
  uploadActions,
  onCreateFolder,
  onViewModeChange,
  onSortChange,
}: {
  selectedCount: number;
  viewMode: DriveViewMode;
  sort: DriveSortKey;
  uploadActions?: ReactNode;
  onCreateFolder: () => void;
  onViewModeChange: (viewMode: DriveViewMode) => void;
  onSortChange: (sort: DriveSortKey) => void;
}) {
  return (
    <Card className="flex flex-col gap-3 p-3 md:flex-row md:items-center md:justify-between">
      <div className="flex flex-wrap items-center gap-2">
        {uploadActions ?? (
          <Button
            type="button"
            variant="outline"
            disabled
            aria-disabled="true"
            title="Import disponible dans une prochaine tache"
          >
            <Upload data-icon="inline-start" aria-hidden="true" />
            Importer bientot
          </Button>
        )}
        <Button type="button" onClick={onCreateFolder}>
          <FolderPlus data-icon="inline-start" aria-hidden="true" />
          Nouveau dossier
        </Button>
        {selectedCount > 0 ? (
          <Badge variant="secondary">
            {selectedCount} selectionne{selectedCount > 1 ? 's' : ''}
          </Badge>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Label className="text-muted-foreground" htmlFor="drive-files-sort">
          Trier
        </Label>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button id="drive-files-sort" type="button" variant="outline">
              {SORT_OPTIONS.find((option) => option.value === sort)?.label ?? 'Trier'}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            <DropdownMenuRadioGroup
              value={sort}
              onValueChange={(value) => onSortChange(value as DriveSortKey)}
            >
              {SORT_OPTIONS.map((option) => (
                <DropdownMenuRadioItem key={option.value} value={option.value}>
                  {option.label}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <ToggleGroup
          type="single"
          value={viewMode}
          onValueChange={(value) => {
            if (value === 'grid' || value === 'list') {
              onViewModeChange(value);
            }
          }}
          variant="outline"
          size="sm"
          spacing={0}
          aria-label="Mode d'affichage"
        >
          <ToggleGroupItem value="list" aria-label="Vue tableau">
            <List aria-hidden="true" />
          </ToggleGroupItem>
          <ToggleGroupItem value="grid" aria-label="Vue grille">
            <Grid2X2 aria-hidden="true" />
          </ToggleGroupItem>
        </ToggleGroup>
      </div>
    </Card>
  );
}
