import { Plus, ShieldAlert, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { ServiceAccount } from '../identity.service-accounts.api';

export function DetailHeaderActions({
  selectedServiceAccount,
  onToggleLifecycle,
  onOpenCreateClient,
}: {
  selectedServiceAccount: ServiceAccount | null;
  onToggleLifecycle: () => void;
  onOpenCreateClient: () => void;
}) {
  if (!selectedServiceAccount) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-2">
      <Button variant="outline" onClick={onToggleLifecycle}>
        {selectedServiceAccount.status === 'active' ? (
          <>
            <ShieldAlert className="size-4" />
            Suspendre
          </>
        ) : (
          <>
            <ShieldCheck className="size-4" />
            Reactiver
          </>
        )}
      </Button>
      <Button onClick={onOpenCreateClient}>
        <Plus className="size-4" />
        Nouveau client
      </Button>
    </div>
  );
}
