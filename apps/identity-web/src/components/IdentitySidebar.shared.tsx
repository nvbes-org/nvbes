import { Link } from '@tanstack/react-router';
import { KeyRound, Link as LinkIcon, Mail, Monitor, Shield } from 'lucide-react';

export const identityNavSections = [
  {
    title: 'Identité',
    items: [
      { to: '/security', icon: Shield, label: 'Sécurité', end: true },
      { to: '/email-addresses', icon: Mail, label: 'Adresses e-mail' },
      { to: '/mfa', icon: KeyRound, label: 'Authentification multifacteur' },
      { to: '/sessions', icon: Monitor, label: 'Appareils et sessions' },
    ],
  },
  {
    title: 'Autorisations',
    items: [{ to: '/linked-apps', icon: LinkIcon, label: 'Applications liées' }],
  },
] as const;

export function SidebarNavItem({
  to,
  icon: Icon,
  label,
  end,
}: {
  to: (typeof identityNavSections)[number]['items'][number]['to'];
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  end?: boolean;
}) {
  return (
    <Link
      to={to}
      activeOptions={{ exact: end }}
      className="flex min-w-0 items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      activeProps={{ className: 'bg-card text-foreground' }}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{label}</span>
    </Link>
  );
}
