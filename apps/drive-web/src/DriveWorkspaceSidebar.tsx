import {
  Clock3,
  CreditCard,
  FileText,
  HardDrive,
  KeyRound,
  Link2,
  Shield,
  Star,
  Trash2,
  User,
  Users,
} from 'lucide-react';
import type { CSSProperties } from 'react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from '@/components/ui/sidebar';
import { useStorageManager } from '@/hooks/use-storage-manager';

const mainNavigation = [
  { label: 'Mes fichiers', icon: FileText, active: true },
  { label: 'Recents', icon: Clock3 },
  { label: 'Favoris', icon: Star },
  { label: 'Partages', icon: Link2 },
  { label: 'Corbeille', icon: Trash2 },
];

const settingsNavigation = [
  { label: 'Membres', icon: Users },
  { label: 'Facturation', icon: CreditCard },
  { label: 'Securite', icon: Shield },
  { label: 'API', icon: KeyRound },
  { label: 'Compte', icon: User },
];

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
      <SidebarHeader className="h-14 justify-center border-b px-4">
        <div className="flex items-center gap-3">
          <div className="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <HardDrive />
          </div>
          <span className="text-lg font-semibold tracking-normal">nvbes Drive</span>
        </div>
      </SidebarHeader>

      <SidebarContent className="px-2 py-3">
        <SidebarGroup className="p-0">
          <SidebarGroupContent>
            <SidebarMenu className="gap-1">
              <SidebarMenuItem>
                <Button className="mb-1 w-full justify-start">
                  <FileText data-icon="inline-start" />
                  Nouveau
                </Button>
              </SidebarMenuItem>
              {mainNavigation.map((item) => (
                <SidebarMenuItem key={item.label}>
                  <SidebarMenuButton
                    isActive={item.active}
                    className="h-8 px-2 text-sm font-medium"
                  >
                    <item.icon />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup className="mt-3 p-0">
          <SidebarGroupLabel className="px-2 text-xs font-semibold uppercase tracking-wider">
            Parametres
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu className="gap-1.5">
              {settingsNavigation.map((item) => (
                <SidebarMenuItem key={item.label}>
                  <SidebarMenuButton className="h-8 px-2 text-sm font-medium">
                    <item.icon />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="border-t p-3">
        <div className="flex flex-col gap-2">
          <div className="text-sm font-medium">{workspaceName}</div>
          <div className="flex items-center justify-between gap-4 text-xs">
            <span className="text-muted-foreground">Stockage</span>
            <span className="font-semibold">
              {storage.supported
                ? `${storage.formattedUsage} / ${storage.formattedQuota}`
                : 'Indisponible'}
            </span>
          </div>
          {storage.supported && (
            <Progress value={storage.usageRatio} aria-label="Stockage utilise" />
          )}
          {storage.supported && storage.persisted === false && (
            <button
              type="button"
              onClick={() => void storage.requestPersistence()}
              disabled={storage.isPersisting}
              className="mt-1 rounded-md bg-muted px-2 py-1 text-[10px] font-medium text-muted-foreground transition-colors hover:bg-muted/80 disabled:opacity-50"
            >
              {storage.isPersisting ? 'Verrouillage...' : 'Activer le stockage persistant'}
            </button>
          )}
          {storage.supported && storage.persisted === true && (
            <span className="mt-1 text-[10px] font-medium text-emerald-500">
              Stockage persistant actif
            </span>
          )}
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
