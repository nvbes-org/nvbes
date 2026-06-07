import { FileText, HardDrive } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar';
import { mainNavigation, settingsNavigation } from './DriveWorkspaceSidebar.data';

export function DriveWorkspaceSidebarHeader() {
  return (
    <SidebarHeader className="h-14 justify-center border-b px-4">
      <div className="flex items-center gap-3">
        <div className="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <HardDrive />
        </div>
        <span className="text-lg font-semibold tracking-normal">nvbes Drive</span>
      </div>
    </SidebarHeader>
  );
}

export function DriveWorkspaceSidebarNav() {
  return (
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
                <SidebarMenuButton isActive={item.active} className="h-8 px-2 text-sm font-medium">
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
  );
}
