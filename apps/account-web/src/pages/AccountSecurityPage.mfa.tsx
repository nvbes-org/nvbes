import { Key, Monitor, ShieldCheck, ShieldOff } from 'lucide-react';

import { Card, CardContent } from '@/components/ui/card';
import { SecurityActionRow } from './AccountSecurityPage.row';
import type { SecurityOverview } from './useAccountSecurityPage';

export function SecurityMfaCard({
  overview,
  disabled,
  onOpenMfa,
  onOpenPassword,
  onOpenSessions,
}: {
  overview: SecurityOverview;
  disabled: boolean;
  onOpenMfa: () => void;
  onOpenPassword: () => void;
  onOpenSessions: () => void;
}) {
  return (
    <Card>
      <CardContent className="flex flex-col">
        <SecurityActionRow
          icon={overview.mfa_enabled ? ShieldCheck : ShieldOff}
          iconColor={overview.mfa_enabled ? 'size-4 text-primary' : 'size-4 text-muted-foreground'}
          label="Double authentification (MFA)"
          description={overview.mfa_enabled ? 'Configurer et activee' : 'Non configuree'}
          badgeLabel={overview.mfa_enabled ? 'Active' : 'Inactive'}
          badgeVariant={overview.mfa_enabled ? 'default' : 'secondary'}
          onAction={onOpenMfa}
          disabled={disabled}
        />

        <SecurityActionRow
          icon={Key}
          label="Mot de passe"
          description="Modifier votre mot de passe"
          onAction={onOpenPassword}
          disabled={disabled}
        />

        <SecurityActionRow
          icon={Monitor}
          label="Sessions actives"
          description={`${overview.session_count} session(s) ouverte(s)`}
          badgeLabel={overview.session_count.toString()}
          badgeVariant="default"
          onAction={onOpenSessions}
          disabled={disabled}
        />
      </CardContent>
    </Card>
  );
}
