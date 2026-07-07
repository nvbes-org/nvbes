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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Spinner } from '@/components/ui/spinner';
import { Textarea } from '@/components/ui/textarea';
import { DialogError } from './WorkspaceServiceAccountsDialogs.shared';

export function WorkspaceServiceAccountsCreateDialog({
  createOpen,
  setCreateOpen,
  createName,
  setCreateName,
  createDescription,
  setCreateDescription,
  createRole,
  setCreateRole,
  createError,
  busyAction,
  onCreate,
  onClearCreate,
}: {
  createOpen: boolean;
  setCreateOpen: (open: boolean) => void;
  createName: string;
  setCreateName: (value: string) => void;
  createDescription: string;
  setCreateDescription: (value: string) => void;
  createRole: string;
  setCreateRole: (value: string) => void;
  createError: string | null;
  busyAction: string | null;
  onCreate: () => void;
  onClearCreate: () => void;
}) {
  return (
    <Dialog
      open={createOpen}
      onOpenChange={(open) => {
        setCreateOpen(open);
        if (!open) onClearCreate();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Creer un service account</DialogTitle>
          <DialogDescription>
            Cree un principal machine workspace-scopé avec un role RBAC initial.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="create-service-account-name">Nom</Label>
            <Input
              id="create-service-account-name"
              value={createName}
              onChange={(event) => setCreateName(event.target.value)}
              placeholder="Drive automation"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="create-service-account-description">Description</Label>
            <Textarea
              id="create-service-account-description"
              className="min-h-24"
              value={createDescription}
              onChange={(event) => setCreateDescription(event.target.value)}
              placeholder="Provisionnement M2M pour Drive"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="create-service-account-role">Role initial</Label>
            <Select value={createRole} onValueChange={setCreateRole}>
              <SelectTrigger id="create-service-account-role" className="w-full">
                <SelectValue placeholder="Choisir un role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="viewer">viewer</SelectItem>
                <SelectItem value="member">member</SelectItem>
                <SelectItem value="admin">admin</SelectItem>
                <SelectItem value="owner">owner</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <DialogError message={createError} />
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => {
              setCreateOpen(false);
              onClearCreate();
            }}
            disabled={busyAction === 'create-service-account'}
          >
            Annuler
          </Button>
          <Button onClick={onCreate} disabled={busyAction === 'create-service-account'}>
            {busyAction === 'create-service-account' ? (
              <>
                <Spinner />
                Creation...
              </>
            ) : (
              'Creer'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
