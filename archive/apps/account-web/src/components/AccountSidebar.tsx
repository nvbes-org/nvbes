import { Link } from '@tanstack/react-router';
import {
  Bell,
  Eye,
  KeyRound,
  Link2,
  Mail,
  MonitorSmartphone,
  Settings,
  UserCircle,
} from 'lucide-react';
import { ACCOUNT_WEB_PATHS } from '@/account.routes';

const navigation = [
  { to: ACCOUNT_WEB_PATHS.profile, icon: UserCircle, label: 'Profil' },
  { to: ACCOUNT_WEB_PATHS.preferences, icon: Settings, label: 'Préférences' },
  { to: ACCOUNT_WEB_PATHS.notifications, icon: Bell, label: 'Notifications' },
  { to: ACCOUNT_WEB_PATHS.privacy, icon: Eye, label: 'Confidentialité' },
  { to: ACCOUNT_WEB_PATHS.security, icon: KeyRound, label: 'Sécurité' },
  { to: ACCOUNT_WEB_PATHS.sessions, icon: MonitorSmartphone, label: 'Sessions' },
  { to: ACCOUNT_WEB_PATHS.emails, icon: Mail, label: 'Adresses e-mail' },
  { to: ACCOUNT_WEB_PATHS.connectedApps, icon: Link2, label: 'Applications connectées' },
] as const;

export function AccountSidebar() {
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
    </nav>
  );
}
