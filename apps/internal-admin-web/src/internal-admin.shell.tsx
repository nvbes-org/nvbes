import {
  Activity,
  Banknote,
  Building2,
  Code2,
  FileCheck2,
  FileDown,
  Globe2,
  IdCard,
  KeyRound,
  LifeBuoy,
  Mail,
  Scale,
  Search,
  Shield,
  UserRound,
  Users,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

const navItems = [
  { href: '#command-center', label: 'Command', icon: Building2 },
  { href: '#revenue-center', label: 'Revenue', icon: Banknote },
  { href: '#customer-center', label: 'Customer', icon: Users },
  { href: '#developer-center', label: 'Developer', icon: Code2 },
  { href: '#operations-center', label: 'Operations', icon: Activity },
  { href: '#access-center', label: 'Access', icon: KeyRound },
  { href: '#audit-evidence-center', label: 'Evidence', icon: FileCheck2 },
  { href: '#identity-governance-center', label: 'Governance', icon: IdCard },
  { href: '#security-center', label: 'Security', icon: Shield },
  { href: '#compliance-center', label: 'Compliance', icon: Scale },
  { href: '#communications-center', label: 'Comms', icon: Mail },
  { href: '#region-center', label: 'Region', icon: Globe2 },
  { href: '#global-search', label: 'Global Search', icon: Search },
  { href: '#tenant-detail', label: 'Tenant', icon: Building2 },
  { href: '#workspace-detail', label: 'Workspace', icon: Building2 },
  { href: '#user-detail', label: 'User', icon: UserRound },
  { href: '#recherche', label: 'Recherche', icon: Search },
  { href: '#provider-events', label: 'Provider Events', icon: Activity },
  { href: '#audit', label: 'Audit', icon: Shield },
  { href: '#mutations', label: 'Mutations', icon: Banknote },
  { href: '#exports', label: 'Exports', icon: FileDown },
  { href: '#runbooks', label: 'Runbooks', icon: LifeBuoy },
];

export function InternalAdminShell({ children }: { children: ReactNode }) {
  return (
    <div className="bg-background text-foreground min-h-screen">
      <aside className="bg-sidebar text-sidebar-foreground border-sidebar-border fixed inset-y-0 left-0 hidden w-64 border-r lg:block">
        <div className="border-sidebar-border flex h-16 items-center gap-3 border-b px-5">
          <div className="bg-sidebar-primary text-sidebar-primary-foreground flex size-9 items-center justify-center rounded-md">
            <Shield className="size-4" />
          </div>
          <div>
            <div className="text-sm font-semibold">nvbes Internal</div>
            <div className="text-sidebar-foreground/60 text-xs">Back-office prive</div>
          </div>
        </div>
        <nav className="space-y-1 p-3">
          {navItems.map((item, index) => (
            <a
              className={cn(
                'flex h-9 items-center gap-3 rounded-md px-3 text-sm transition-colors',
                index === 0
                  ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                  : 'text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
              )}
              href={item.href}
              key={item.label}
            >
              <item.icon className="size-4" />
              {item.label}
            </a>
          ))}
        </nav>
      </aside>

      <main className="lg:pl-64">
        <header className="bg-background/95 border-border sticky top-0 z-10 border-b backdrop-blur">
          <div className="flex min-h-16 items-center justify-between gap-4 px-4 lg:px-8">
            <div>
              <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
                Back-office
              </p>
              <h1 className="text-lg font-semibold">Pilotage back-office</h1>
            </div>
            <div className="border-border bg-card hidden items-center gap-2 rounded-md border px-3 py-2 text-xs md:flex">
              <Activity className="text-primary size-4" />
              Actions auditees
            </div>
          </div>
          <nav className="scrollbar-none border-border flex gap-2 overflow-x-auto border-t px-4 py-2 lg:hidden">
            {navItems.map((item) => (
              <a
                className="bg-card text-muted-foreground hover:text-foreground flex h-8 shrink-0 items-center gap-2 rounded-md border px-3 text-xs"
                href={item.href}
                key={item.label}
              >
                <item.icon className="size-3.5" />
                {item.label}
              </a>
            ))}
          </nav>
        </header>
        <div className="mx-auto max-w-[1480px] px-4 py-5 lg:px-8">{children}</div>
      </main>
    </div>
  );
}
