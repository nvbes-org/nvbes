import { KeyRound } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { ServiceAccount } from '../identity.service-accounts.api';
import { formatDateTime, statusLabel } from './WorkspaceServiceAccounts.helpers';

export function ServiceAccountLifecycle({
  selectedServiceAccount,
  onOpenAttachClient,
}: {
  selectedServiceAccount: ServiceAccount;
  onOpenAttachClient: () => void;
}) {
  return (
    <div className="rounded-3xl border border-border/70 bg-background p-5">
      <p className="text-sm font-medium">Cycle de vie</p>
      <div className="mt-4 flex flex-col gap-3 text-sm">
        <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
          <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Cree le</p>
          <p className="mt-1 font-medium">{formatDateTime(selectedServiceAccount.created_at)}</p>
        </div>
        <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
          <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Mis a jour</p>
          <p className="mt-1 font-medium">{formatDateTime(selectedServiceAccount.updated_at)}</p>
        </div>
        <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
          <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Statut</p>
          <p className="mt-1 font-medium">{statusLabel(selectedServiceAccount.status)}</p>
        </div>
        <Button variant="outline" onClick={onOpenAttachClient}>
          <KeyRound className="size-4" />
          Attacher un client existant
        </Button>
      </div>
    </div>
  );
}
