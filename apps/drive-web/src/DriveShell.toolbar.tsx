import { Grid2X2, List } from 'lucide-react';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { DriveUploadActions } from './DriveUploadActions';

export function DriveShellToolbar({
  currentWorkspaceId,
}: {
  currentWorkspaceId: string | undefined;
}) {
  return (
    <section className="border-b bg-background px-4 py-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-base font-semibold">Mes fichiers</h1>
          <p className="text-sm text-muted-foreground">
            Connecte via Identity en OAuth2, sans session locale persistante.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <ToggleGroup
            className="rounded-lg border bg-background p-0.5 shadow-sm"
            defaultValue={['grid']}
          >
            <ToggleGroupItem
              aria-label="Vue grille"
              value="grid"
              className="data-pressed:bg-primary data-pressed:text-primary-foreground"
            >
              <Grid2X2 />
            </ToggleGroupItem>
            <ToggleGroupItem aria-label="Vue liste" value="list">
              <List />
            </ToggleGroupItem>
          </ToggleGroup>
          {currentWorkspaceId ? <DriveUploadActions workspaceId={currentWorkspaceId} /> : null}
        </div>
      </div>
    </section>
  );
}
