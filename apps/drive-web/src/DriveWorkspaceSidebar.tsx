import type { CSSProperties } from 'react';
import { Sidebar, SidebarProvider } from '@/components/ui/sidebar';
import { useStorageManager } from '@/hooks/use-storage-manager';
import { DriveWorkspaceSidebarFooter } from './DriveWorkspaceSidebar.footer';
import { DriveWorkspaceSidebarHeader, DriveWorkspaceSidebarNav } from './DriveWorkspaceSidebar.nav';

export function DriveSidebarProvider({ children }: { children: React.ReactNode }) {
  return (
    <SidebarProvider style={{ '--sidebar-width': '16rem' } as CSSProperties}>
      {children}
    </SidebarProvider>
  );
}

export function DriveWorkspaceSidebar({ workspaceName }: { workspaceName: string }) {
  const storage = useStorageManager();

  return (
    <Sidebar collapsible="none" className="h-svh border-r bg-background">
      <DriveWorkspaceSidebarHeader />
      <DriveWorkspaceSidebarNav />
      <DriveWorkspaceSidebarFooter workspaceName={workspaceName} storage={storage} />
    </Sidebar>
  );
}
