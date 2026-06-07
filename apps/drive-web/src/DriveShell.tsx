import { useDeferredValue, useState } from 'react';
import { SidebarInset } from '@/components/ui/sidebar';
import { type DriveFileItem, DriveFileList } from './DriveFileList';
import { DriveSidebarProvider, DriveWorkspaceSidebar } from './DriveWorkspaceSidebar';
import type { DriveMeResponse } from './drive.api';
import { DriveSessionContextCard, DriveShellHeader, DriveShellToolbar } from './DriveShell.shared';

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
    <DriveSidebarProvider>
      <DriveWorkspaceSidebar workspaceName={currentWorkspace?.name ?? 'Workspace personnel'} />
      <SidebarInset className="min-h-svh">
        <DriveShellHeader
          accessToken={accessToken}
          user={user}
          search={search}
          deferredSearch={deferredSearch}
          onSearchChange={setSearch}
        />

        <main className="flex flex-1 flex-col bg-muted/30">
          <DriveShellToolbar currentWorkspaceId={currentWorkspace?.id} />

          <section className="grid flex-1 gap-4 p-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(18rem,0.7fr)]">
            <DriveFileList files={visibleFiles} query={deferredSearch} />
            <DriveSessionContextCard user={user} currentWorkspace={currentWorkspace} />
          </section>
        </main>
      </SidebarInset>
    </DriveSidebarProvider>
  );
}
