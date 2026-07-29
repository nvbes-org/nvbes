import { accountNavSections, SidebarNavItem } from '@/components/AccountSidebar.shared';
import { useAuthuser } from '@/hooks/useAuthuser';

export function AccountSidebar() {
  const authuser = useAuthuser();

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="flex-1 overflow-y-auto px-2 pt-4">
        <nav className="flex min-w-0 flex-col gap-5">
          {accountNavSections.map((section) => (
            <div key={section.title} className="flex min-w-0 flex-col gap-1">
              <span className="px-2.5 text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-[--secondary-background] truncate">
                {section.title}
              </span>
              <div className="mt-1 flex flex-col gap-0.5">
                {section.items.map((item) => (
                  <SidebarNavItem
                    key={item.to}
                    to={item.to}
                    icon={item.icon}
                    label={item.label}
                    end={'end' in item ? item.end : undefined}
                    authuser={authuser}
                  />
                ))}
              </div>
            </div>
          ))}
        </nav>
      </div>
    </div>
  );
}
