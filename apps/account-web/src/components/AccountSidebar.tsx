import { Link } from '@tanstack/react-router';
import { Bell, Eye, Settings, ShieldCheck, UserCircle } from 'lucide-react';
import { ACCOUNT_WEB_PATHS } from '@/account.routes';

const navigation = [
  { to: ACCOUNT_WEB_PATHS.profile, icon: UserCircle, label: 'Profil' },
  { to: ACCOUNT_WEB_PATHS.preferences, icon: Settings, label: 'Préférences' },
  { to: ACCOUNT_WEB_PATHS.notifications, icon: Bell, label: 'Notifications' },
  { to: ACCOUNT_WEB_PATHS.privacy, icon: Eye, label: 'Confidentialité' },
] as const;

export function AccountSidebar() {
  const securityUrl = identitySecurityUrl();

  return (
    <nav aria-label="Navigation du compte" className="sticky top-14 flex flex-col gap-1 p-4">
      {navigation.map(({ to, icon: Icon, label }) => (
        <Link
          key={to}
          to={to}
          activeOptions={{ exact: true }}
          className="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          activeProps={{ className: 'bg-card text-foreground shadow-sm' }}
        >
          <Icon className="size-4" aria-hidden="true" />
          {label}
        </Link>
      ))}
      {securityUrl ? (
        <a
          href={securityUrl}
          className="mt-3 flex items-center gap-3 border-t border-border px-3 pt-4 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
        >
          <ShieldCheck className="size-4" aria-hidden="true" />
          Sécurité dans Identity
        </a>
      ) : null}
    </nav>
  );
}

export function identitySecurityUrl(): string | null {
  const baseUrl = import.meta.env.VITE_IDENTITY_WEB_BASE_URL?.trim();
  if (!baseUrl) {
    return null;
  }
  try {
    const url = new URL('/security', baseUrl);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.toString() : null;
  } catch {
    return null;
  }
}
