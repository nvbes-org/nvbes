import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';

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
          <DialogTitle>Confirmer la suppression</DialogTitle>
          <DialogDescription>
            Cette action est irréversible. Votre compte, vos espaces et vos données seront
            définitivement supprimés.
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field data-invalid={Boolean(deleteError)}>
            <FieldLabel htmlFor="delete-account-confirmation">
              Tapez SUPPRIMER pour confirmer
            </FieldLabel>
            <Input
              id="delete-account-confirmation"
              type="text"
              placeholder="SUPPRIMER"
              value={deleteConfirmText}
              onChange={(event) => onDeleteConfirmTextChange(event.target.value)}
              aria-invalid={Boolean(deleteError)}
              autoComplete="off"
              autoFocus
            />
            <FieldDescription>
              Cette confirmation évite une suppression accidentelle.
            </FieldDescription>
            <FieldError>{deleteError}</FieldError>
          </Field>
        </FieldGroup>
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
            {deleting && <Spinner data-icon="inline-start" />}
            {deleting ? 'Suppression…' : 'Supprimer définitivement'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
