import { Link } from '@tanstack/react-router';
import { Bell, Eye, Link as LinkIcon, Monitor, Settings, Shield, UserCircle } from 'lucide-react';

import { accountPathForAuthuser } from '@/identity.authuser';

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
    ],
  },
  {
    title: 'Parametres',
    items: [
      { to: '/account/notifications', icon: Bell, label: 'Notifications' },
      { to: '/account/preferences', icon: Settings, label: 'Preferences' },
    ],
  },
] as const;

export function SidebarNavItem({
  to,
  icon: Icon,
  label,
  end,
  authuser,
}: {
  to: string;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  end?: boolean;
  authuser: string;
}) {
  return (
    <Link
      to={accountPathForAuthuser(authuser, to)}
      activeOptions={{ exact: end }}
      className="flex min-w-0 items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      activeProps={{ className: 'bg-card text-foreground' }}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{label}</span>
    </Link>
  );
}
