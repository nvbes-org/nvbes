import { Link } from '@tanstack/react-router';
import { Bell, Eye, Settings, UserCircle } from 'lucide-react';
import { ACCOUNT_WEB_PATHS } from '@/account.routes';

const mobileNavigation = [
  { to: ACCOUNT_WEB_PATHS.profile, label: 'Profil', icon: UserCircle },
  { to: ACCOUNT_WEB_PATHS.preferences, label: 'Préférences', icon: Settings },
  { to: ACCOUNT_WEB_PATHS.notifications, label: 'Notifications', icon: Bell },
  { to: ACCOUNT_WEB_PATHS.privacy, label: 'Confidentialité', icon: Eye },
] as const;

export function AccountTopBar() {
  return (
    <header className="sticky top-0 z-20 border-b border-border bg-background/95 backdrop-blur">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-4">
        <Link
          to={ACCOUNT_WEB_PATHS.profile}
          className="text-sm font-semibold tracking-tight text-foreground"
        >
          nvbes <span className="text-muted-foreground">Account</span>
        </Link>
        <nav aria-label="Navigation mobile" className="flex items-center gap-1 md:hidden">
          {mobileNavigation.map(({ to, label, icon: Icon }) => (
            <Link
              key={to}
              to={to}
              title={label}
              aria-label={label}
              activeOptions={{ exact: true }}
              className="rounded-lg p-2 text-muted-foreground hover:bg-muted hover:text-foreground"
              activeProps={{ className: 'bg-card text-foreground' }}
            >
              <Icon className="size-4" aria-hidden="true" />
            </Link>
          ))}
        </nav>
      </div>
    </header>
  );
}
