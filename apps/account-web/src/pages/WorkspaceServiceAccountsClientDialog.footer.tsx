import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function WorkspaceServiceAccountsClientDialogFooter({
  busyAction,
  onCancel,
  onCreateClient,
}: {
  busyAction: string | null;
  onCancel: () => void;
  onCreateClient: () => void;
}) {
  return (
    <>
      <Button variant="outline" onClick={onCancel} disabled={busyAction === 'create-client'}>
        Annuler
      </Button>
      <Button onClick={onCreateClient} disabled={busyAction === 'create-client'}>
        {busyAction === 'create-client' ? (
          <>
            <Loader2 className="size-4 animate-spin" />
            Creation...
          </>
        ) : (
          'Creer le client'
        )}
      </Button>
    </>
  );
}
