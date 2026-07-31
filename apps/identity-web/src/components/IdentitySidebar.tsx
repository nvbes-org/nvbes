import { identityNavSections, SidebarNavItem } from '@/components/IdentitySidebar.shared';

export function IdentitySidebar() {
  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="flex-1 overflow-y-auto px-2 pt-4">
        <nav className="flex min-w-0 flex-col gap-5">
          {identityNavSections.map((section) => (
            <div key={section.title} className="flex min-w-0 flex-col gap-1">
              <span className="truncate px-2.5 text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-[--secondary-background]">
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
