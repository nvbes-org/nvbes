import { Loader2 } from 'lucide-react';
import type { FormEvent } from 'react';

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
import { WorkspacesError } from './WorkspacesPage.layout';

export function CreateWorkspaceDialog({
  open,
  workspaceName,
  creating,
  error,
  onOpenChange,
  onWorkspaceNameChange,
  onSubmit,
}: {
  open: boolean;
  workspaceName: string;
  creating: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
  onWorkspaceNameChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Creer un workspace</DialogTitle>
          <DialogDescription>
            Creez un nouvel espace de travail pour votre equipe ou organisation.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit}>
          <div className="flex flex-col gap-4 py-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="workspace-name">Nom du workspace</Label>
              <Input
                id="workspace-name"
                type="text"
                placeholder="Mon entreprise"
                value={workspaceName}
                onChange={(event) => onWorkspaceNameChange(event.target.value)}
                required
                autoFocus
                maxLength={100}
              />
              <p className="text-xs text-muted-foreground">
                Le nom de votre organisation ou de votre projet.
              </p>
            </div>
            {error && <WorkspacesError message={error} />}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={creating}
            >
              Annuler
            </Button>
            <Button type="submit" disabled={creating}>
              {creating ? (
                <>
                  <Loader2 className="size-4 animate-spin" data-icon="inline-start" />
                  Creation...
                </>
              ) : (
                'Creer'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
