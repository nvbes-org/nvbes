import { FolderPlus, Grid2X2, List, Upload } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from './lib/classnames';
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
  onCreateFolder,
  onViewModeChange,
  onSortChange,
}: {
  selectedCount: number;
  viewMode: DriveViewMode;
  sort: DriveSortKey;
  onCreateFolder: () => void;
  onViewModeChange: (viewMode: DriveViewMode) => void;
  onSortChange: (sort: DriveSortKey) => void;
}) {
  return (
    <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-background/90 p-3 shadow-sm md:flex-row md:items-center md:justify-between">
      <div className="flex flex-wrap items-center gap-2">
        <Button
          type="button"
          variant="outline"
          disabled
          aria-disabled="true"
          title="Import disponible dans une prochaine tache"
        >
          <Upload className="size-4" aria-hidden="true" />
          Importer bientot
        </Button>
        <Button type="button" onClick={onCreateFolder}>
          <FolderPlus className="size-4" aria-hidden="true" />
          Nouveau dossier
        </Button>
        {selectedCount > 0 ? (
          <span className="rounded-full bg-muted px-3 py-1 text-sm text-muted-foreground">
            {selectedCount} selectionne{selectedCount > 1 ? 's' : ''}
          </span>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <label className="text-sm text-muted-foreground" htmlFor="drive-files-sort">
          Trier
        </label>
        <select
          id="drive-files-sort"
          value={sort}
          onChange={(event) => onSortChange(event.target.value as DriveSortKey)}
          className="h-8 rounded-lg border border-border bg-background px-2 text-sm outline-none transition focus:border-ring focus:ring-2 focus:ring-ring/30"
        >
          {SORT_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <div className="flex rounded-lg border border-border bg-background p-0.5">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className={cn(viewMode === 'list' && 'bg-muted text-foreground')}
            aria-label="Vue tableau"
            aria-pressed={viewMode === 'list'}
            onClick={() => onViewModeChange('list')}
          >
            <List className="size-4" aria-hidden="true" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className={cn(viewMode === 'grid' && 'bg-muted text-foreground')}
            aria-label="Vue grille"
            aria-pressed={viewMode === 'grid'}
            onClick={() => onViewModeChange('grid')}
          >
            <Grid2X2 className="size-4" aria-hidden="true" />
          </Button>
        </div>
      </div>
    </div>
  );
}
