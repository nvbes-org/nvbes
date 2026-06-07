import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

import { ErrorMessage } from './AccountPrivacyPage.cards';

export function DeleteAccountDialog({
  open,
  deleting,
  deleteConfirmText,
  deleteError,
  onOpenChange,
  onDeleteConfirmTextChange,
  onDelete,
}: {
  open: boolean;
  deleting: boolean;
  deleteConfirmText: string;
  deleteError: string | null;
  onOpenChange: (open: boolean) => void;
  onDeleteConfirmTextChange: (value: string) => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Supprimer votre compte</DialogTitle>
          <DialogDescription>
            Cette action est irreversible. Toutes vos donnees, workspaces et abonnements seront
            definitivement supprimes. Tapez SUPPRIMER pour confirmer.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-4">
          <input
            type="text"
            className="flex h-10 w-full rounded-lg border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            placeholder="SUPPRIMER"
            value={deleteConfirmText}
            onChange={(event) => onDeleteConfirmTextChange(event.target.value)}
            autoFocus
          />
          {deleteError && <ErrorMessage message={deleteError} />}
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={deleting}
          >
            Annuler
          </Button>
          <Button
            type="button"
            variant="default"
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            onClick={onDelete}
            disabled={deleting || deleteConfirmText !== 'SUPPRIMER'}
          >
            {deleting ? 'Suppression...' : 'Supprimer definitivement'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
