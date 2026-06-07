import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { DialogError } from './WorkspaceServiceAccountsDialogs.shared';

export function WorkspaceServiceAccountsAttachDialog({
  attachDialogOpen,
  setAttachDialogOpen,
  attachClientId,
  setAttachClientId,
  attachError,
  busyAction,
  onAttachClient,
  onClearAttach,
}: {
  attachDialogOpen: boolean;
  setAttachDialogOpen: (open: boolean) => void;
  attachClientId: string;
  setAttachClientId: (value: string) => void;
  attachError: string | null;
  busyAction: string | null;
  onAttachClient: () => void;
  onClearAttach: () => void;
}) {
  return (
    <Dialog
      open={attachDialogOpen}
      onOpenChange={(open) => {
        setAttachDialogOpen(open);
        if (!open) onClearAttach();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Attacher un client OAuth existant</DialogTitle>
          <DialogDescription>Rattache un client deja cree a ce service account.</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="attach-client-id">client_id</Label>
            <Input
              id="attach-client-id"
              value={attachClientId}
              onChange={(event) => setAttachClientId(event.target.value)}
              placeholder="client_..."
            />
          </div>
          <DialogError message={attachError} />
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => {
              setAttachDialogOpen(false);
              onClearAttach();
            }}
            disabled={busyAction === 'attach-client'}
          >
            Annuler
          </Button>
          <Button onClick={onAttachClient} disabled={busyAction === 'attach-client'}>
            {busyAction === 'attach-client' ? (
              <>
                <Loader2 className="size-4 animate-spin" />
                Attachement...
              </>
            ) : (
              'Attacher'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
