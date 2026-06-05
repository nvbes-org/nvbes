import { Link } from '@tanstack/react-router';
import {
  BadgeCheck,
  Bell,
  Building,
  CreditCard,
  Eye,
  FileSearch,
  Globe,
  KeyRound,
  Link as LinkIcon,
  LogOut,
  Monitor,
  Receipt,
  Settings,
  Shield,
  UserCircle,
} from 'lucide-react';

import { AccountChooser } from '@/components/AccountChooser';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import type { AccountEntry } from '@/lib/account-context';
import { canManageServiceAccounts } from '@/lib/workspace-permissions';

const navSections = [
  {
    title: 'Compte',
    items: [
      { to: '/account', icon: UserCircle, label: 'Informations personnelles', end: true },
      { to: '/account/security', icon: Shield, label: 'Securite' },
      { to: '/account/sessions', icon: Monitor, label: 'Appareils & sessions' },
    ],
  },
  {
    title: 'Vie privee & donnees',
    items: [
      { to: '/account/privacy', icon: Eye, label: 'Consentements' },
      { to: '/account/linked-apps', icon: LinkIcon, label: 'Apps liees' },
      { to: '/account/audits', icon: FileSearch, label: 'Audits & RGPD' },
    ],
  },
  {
    title: 'Organisation',
    items: [
      { to: '/account/workspaces', icon: Building, label: 'Workspaces' },
      { to: '/account/workspaces/service-accounts', icon: KeyRound, label: 'Comptes de service' },
      { to: '/account/billing', icon: CreditCard, label: 'Facturation' },
      { to: '/account/subscriptions', icon: Receipt, label: 'Abonnements' },
      { to: '/account/standing', icon: BadgeCheck, label: 'Statut du compte' },
    ],
  },
  {
    title: 'Parametres',
    items: [
      { to: '/account/notifications', icon: Bell, label: 'Notifications' },
      { to: '/account/social', icon: Globe, label: 'Contenu & social' },
      { to: '/account/preferences', icon: Settings, label: 'Preferences' },
    ],
  },
];

function SidebarNavItem({
  to,
  icon: Icon,
  label,
  end,
}: {
  to: string;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  end?: boolean;
}) {
  return (
    <Link
      to={to}
      activeOptions={{ exact: end }}
      className="flex min-w-0 items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      activeProps={{ className: 'bg-muted text-foreground' }}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{label}</span>
    </Link>
  );
}

interface AccountSidebarProps {
  accounts: AccountEntry[];
  loadingAccounts: boolean;
  currentWorkspaceRole?: string | null;
  onLogout: () => void;
}

export function AccountSidebar({
  accounts,
  loadingAccounts,
  currentWorkspaceRole,
  onLogout,
}: AccountSidebarProps) {
  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <AccountChooser accounts={accounts} loading={loadingAccounts} />

      <ScrollArea className="flex-1 overflow-hidden px-2">
        <nav className="flex min-w-0 flex-col gap-5">
          {navSections.map((section) => (
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
                      end={item.end}
                    />
                  ))}
              </div>
            </div>
          ))}
        </nav>
      </ScrollArea>

      <Separator />

      <div className="border-t border-sidebar-border/70 p-3">
        <Button
          variant="ghost"
          className="w-full justify-start gap-2.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          size="sm"
          onClick={onLogout}
        >
          <LogOut className="size-4" />
          Se deconnecter
        </Button>
      </div>
    </div>
  );
}
