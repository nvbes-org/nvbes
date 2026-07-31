import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function IdentityPasswordSessionDialog({
  open,
  onKeepSessions,
  onRevokeSessions,
}: {
  open: boolean;
  onKeepSessions: () => void;
  onRevokeSessions: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onKeepSessions()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Déconnecter les autres sessions ?</DialogTitle>
          <DialogDescription>
            Votre mot de passe a été modifié. Vous pouvez maintenant déconnecter tous les autres
            appareils connectés à votre compte.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onKeepSessions}>
            Garder les sessions
          </Button>
          <Button type="button" onClick={onRevokeSessions}>
            Déconnecter les autres
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
