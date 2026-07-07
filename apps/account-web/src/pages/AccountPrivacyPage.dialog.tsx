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
          <Input
            type="text"
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
            variant="destructive"
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
