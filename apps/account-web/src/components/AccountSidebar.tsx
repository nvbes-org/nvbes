import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { canManageServiceAccounts } from '@/lib/workspace-permissions';
import {
  AccountSidebarLogout,
  accountNavSections,
  SidebarNavItem,
} from '@/components/AccountSidebar.shared';

interface AccountSidebarProps {
  currentWorkspaceRole?: string | null;
  onLogout: () => void;
}

export function AccountSidebar({ currentWorkspaceRole, onLogout }: AccountSidebarProps) {
  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <ScrollArea className="flex-1 overflow-hidden px-2 pt-4">
        <nav className="flex min-w-0 flex-col gap-5">
          {accountNavSections.map((section) => (
            <div key={section.title} className="flex min-w-0 flex-col gap-1">
              <span className="px-2.5 text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground truncate">
                {section.title}
              </span>
              <div className="mt-1 flex flex-col gap-0.5">
                {section.items
                  .filter(
                    (item) =>
                      item.to !== '/account/workspaces/service-accounts' ||
                      canManageServiceAccounts(currentWorkspaceRole),
                  )
                  .map((item) => (
                    <SidebarNavItem
                      key={item.to}
                      to={item.to}
                      icon={item.icon}
                      label={item.label}
                      end={'end' in item ? item.end : undefined}
                    />
                  ))}
              </div>
            </div>
          ))}
        </nav>
      </ScrollArea>

      <Separator />

      <AccountSidebarLogout onLogout={onLogout} />
    </div>
  );
}
