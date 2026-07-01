import { Link, useLocation } from '@tanstack/react-router';
import {
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
  ShieldCheck,
  UserCircle,
} from 'lucide-react';

import { Button } from '@/components/ui/button';
import { authuserSearch, readAuthuser } from '@/identity.authuser';

export const accountNavSections = [
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
      { to: '/account/trust-center', icon: ShieldCheck, label: 'Trust Center' },
    ],
  },
  {
    title: 'Organisation',
    items: [
      { to: '/account/workspaces', icon: Building, label: 'Workspaces' },
      { to: '/account/workspaces/service-accounts', icon: KeyRound, label: 'Comptes de service' },
      { to: '/account/billing', icon: CreditCard, label: 'Facturation' },
      { to: '/account/subscriptions', icon: Receipt, label: 'Abonnements' },
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
] as const;

export function SidebarNavItem({
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
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);

  return (
    <Link
      to={to}
      search={authuserSearch(authuser)}
      activeOptions={{ exact: end }}
      className="flex min-w-0 items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      activeProps={{ className: 'bg-muted text-foreground' }}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{label}</span>
    </Link>
  );
}

export function AccountSidebarLogout({ onLogout }: { onLogout: () => void }) {
  return (
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
  );
}
