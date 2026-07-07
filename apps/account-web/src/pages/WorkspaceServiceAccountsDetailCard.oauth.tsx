import { KeyRound, Plus } from 'lucide-react';

import { Button } from '@/components/ui/button';
import type { ServiceAccount, ServiceAccountClient } from '../identity.service-accounts.api';
import { OAuthClientsList } from './WorkspaceServiceAccountsDetailCard.oauth.list';

export function OAuthClientsSection({
  selectedServiceAccount,
  onOpenAttachClient,
  onOpenCreateClient,
  onRotateClientSecret,
  onRevokeClient,
}: {
  selectedServiceAccount: ServiceAccount;
  onOpenAttachClient: () => void;
  onOpenCreateClient: () => void;
  onRotateClientSecret: (client: ServiceAccountClient) => void;
  onRevokeClient: (client: ServiceAccountClient) => void;
}) {
  return (
    <div>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h3 className="text-base font-semibold">Clients OAuth</h3>
          <p className="text-sm text-muted-foreground">
            Secrets visibles une seule fois, rotation/revocation immediates.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={onOpenAttachClient}>
            <KeyRound className="size-4" />
            Attacher un client
          </Button>
          <Button onClick={onOpenCreateClient}>
            <Plus className="size-4" />
            Creer un client
          </Button>
        </div>
      </div>

      <OAuthClientsList
        selectedServiceAccount={selectedServiceAccount}
        onRotateClientSecret={onRotateClientSecret}
        onRevokeClient={onRevokeClient}
      />
    </div>
  );
}
