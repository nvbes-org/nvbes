import { Bell, FileText, Grid2X2, HelpCircle, List, Upload } from 'lucide-react';
import type { ReactNode } from 'react';
import { useDeferredValue, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { SidebarInset } from '@/components/ui/sidebar';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { DriveAccountMenu } from './DriveAccountMenu';
import { type DriveFileItem, DriveFileList } from './DriveFileList';
import { DriveSidebarProvider, DriveWorkspaceSidebar } from './DriveWorkspaceSidebar';
import type { DriveMeResponse } from './drive.api';
import { DriveUploadActions } from './DriveUploadActions';

const files: DriveFileItem[] = [
  { name: 'Documents', detail: '-', type: 'folder' },
  { name: 'Images', detail: '-', type: 'folder' },
  {
    name: 'Presentation_Client.pptx',
    detail: '12 Mo',
    type: 'document',
    shareUrl: 'https://nvbes.app/s/abc123',
  },
  {
    name: 'Backup_Base_Donnees.sql',
    detail: '256 Mo',
    type: 'code',
    shareUrl: 'https://nvbes.app/s/def456',
  },
];

export function DriveShell({ accessToken, me }: { accessToken: string; me: DriveMeResponse }) {
  const [search, setSearch] = useState('');
  const deferredSearch = useDeferredValue(search);
  const user = me.user;
  const currentWorkspace =
    me.workspaces.find((workspace) => workspace.id === me.current_workspace_id) ?? me.workspaces[0];
  const normalizedSearch = deferredSearch.trim().toLowerCase();
  const visibleFiles = normalizedSearch
    ? files.filter((file) => file.name.toLowerCase().includes(normalizedSearch))
    : files;

  return (
    <TooltipProvider>
      <DriveSidebarProvider>
        <DriveWorkspaceSidebar workspaceName={currentWorkspace?.name ?? 'Workspace personnel'} />
        <SidebarInset className="min-h-svh">
          <header className="flex h-14 items-center justify-between border-b px-4">
            <div className="relative w-full max-w-[400px]">
              <FileText className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <Input
                className="h-9 bg-background pl-10 text-sm shadow-sm"
                aria-label="Rechercher dans Mes fichiers"
                placeholder="Rechercher dans Mes fichiers (appuyez sur /)"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
              {deferredSearch !== search ? (
                <span className="absolute inset-y-0 right-3 flex items-center text-xs text-muted-foreground">
                  Filtrage...
                </span>
              ) : null}
            </div>
            <div className="flex items-center gap-2">
              <IconButton label="Aide">
                <HelpCircle />
              </IconButton>
              <IconButton label="Notifications">
                <Bell />
              </IconButton>

              <DriveAccountMenu accessToken={accessToken} user={user} />
            </div>
          </header>

          <main className="flex flex-1 flex-col bg-muted/30">
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
                  {currentWorkspace && <DriveUploadActions workspaceId={currentWorkspace.id} />}
                </div>
              </div>
            </section>

            <section className="grid flex-1 gap-4 p-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(18rem,0.7fr)]">
              <DriveFileList files={visibleFiles} query={deferredSearch} />

              <Card className="border-border/60 shadow-sm">
                <CardHeader className="space-y-1">
                  <CardTitle className="text-base">Contexte de session</CardTitle>
                  <CardDescription>
                    Utilisateur et workspace recuperes via `/auth/me`.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4 text-sm">
                  <StatRow label="Utilisateur" value={user.display_name} />
                  <StatRow label="Email" value={user.email} />
                  <StatRow label="Workspace" value={currentWorkspace?.name ?? 'Aucun'} />
                  <StatRow label="Role" value={currentWorkspace?.role ?? 'n/a'} />
                  <div className="rounded-2xl border border-border/60 bg-muted/40 p-4">
                    <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.28em] text-muted-foreground">
                      <Upload className="size-3.5" />
                      Upload natif
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Utilisez le bouton "Ouvrir" pour transferer des fichiers depuis votre appareil
                      via l'API File System Access.
                    </p>
                  </div>
                </CardContent>
              </Card>
            </section>
          </main>
        </SidebarInset>
      </DriveSidebarProvider>
    </TooltipProvider>
  );
}

function IconButton({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button variant="outline" aria-label={label} className="size-9 rounded-full p-0">
            {children}
          </Button>
        }
      />
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/60 bg-background px-4 py-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-[14rem] truncate font-medium">{value}</span>
    </div>
  );
}
