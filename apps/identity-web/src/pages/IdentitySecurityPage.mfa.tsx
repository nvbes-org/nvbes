import { Key, Monitor, ShieldCheck, ShieldOff } from 'lucide-react';

import { IdentityActionItem } from '@/components/IdentityActionItem';
import { Card, CardContent } from '@/components/ui/card';
import type { SecurityOverview } from './useIdentitySecurityPage';

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
        <IdentityActionItem
          icon={overview.mfa_enabled ? ShieldCheck : ShieldOff}
          iconClassName={
            overview.mfa_enabled ? 'size-4 text-primary' : 'size-4 text-muted-foreground'
          }
          label="Passkeys et authentification multifacteur"
          description={overview.mfa_enabled ? 'Configurer et activee' : 'Non configuree'}
          badgeLabel={overview.mfa_enabled ? 'Active' : 'Inactive'}
          badgeVariant={overview.mfa_enabled ? 'default' : 'secondary'}
          onAction={onOpenMfa}
          disabled={disabled}
        />

        <IdentityActionItem
          icon={Key}
          label="Mot de passe"
          description="Modifier votre mot de passe"
          onAction={onOpenPassword}
          disabled={disabled}
        />

        <IdentityActionItem
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
