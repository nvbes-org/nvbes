import { Copy, Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Badge } from '@/components/ui/badge';
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
import { formatDateTime, type SecretResult } from './WorkspaceServiceAccounts.helpers';

function DialogError({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function SecretDialog({
  result,
  open,
  onOpenChange,
}: {
  result: SecretResult | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!open) setCopied(false);
  }, [open]);

  const handleCopy = async () => {
    if (!result) return;
    await navigator.clipboard.writeText(result.client_secret);
    setCopied(true);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Secret OAuth genere</DialogTitle>
          <DialogDescription>
            Le secret n&apos;est affiche qu&apos;une seule fois. Copiez-le maintenant.
          </DialogDescription>
        </DialogHeader>
        {result && (
          <div className="flex flex-col gap-4">
            <div className="rounded-2xl border border-border/70 bg-muted/40 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                    Client
                  </p>
                  <p className="mt-1 text-sm font-medium">{result.client_id}</p>
                </div>
                <Badge variant="secondary">
                  {result.kind === 'created' ? 'Creation' : 'Rotation'} ·{' '}
                  {formatDateTime(result.timestamp)}
                </Badge>
              </div>
              <div className="mt-4 rounded-xl border border-border/70 bg-background p-3 font-mono text-sm break-all">
                {result.client_secret}
              </div>
            </div>
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={handleCopy} disabled={!result}>
            <Copy className="size-4" />
            {copied ? 'Copie' : 'Copier'}
          </Button>
          <Button onClick={() => onOpenChange(false)}>Fermer</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function WorkspaceServiceAccountsDialogs({
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
  clientDialogOpen,
  setClientDialogOpen,
  clientName,
  setClientName,
  clientScopes,
  setClientScopes,
  clientAudiences,
  setClientAudiences,
  clientResources,
  setClientResources,
  clientRequiredAcr,
  setClientRequiredAcr,
  clientAssertionRequired,
  setClientAssertionRequired,
  clientAssertionJwk,
  setClientAssertionJwk,
  clientError,
  onCreateClient,
  onClearClient,
  attachDialogOpen,
  setAttachDialogOpen,
  attachClientId,
  setAttachClientId,
  attachError,
  onAttachClient,
  onClearAttach,
  secretResult,
  secretDialogOpen,
  setSecretDialogOpen,
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
  clientDialogOpen: boolean;
  setClientDialogOpen: (open: boolean) => void;
  clientName: string;
  setClientName: (value: string) => void;
  clientScopes: string;
  setClientScopes: (value: string) => void;
  clientAudiences: string;
  setClientAudiences: (value: string) => void;
  clientResources: string;
  setClientResources: (value: string) => void;
  clientRequiredAcr: string;
  setClientRequiredAcr: (value: string) => void;
  clientAssertionRequired: boolean;
  setClientAssertionRequired: (value: boolean) => void;
  clientAssertionJwk: string;
  setClientAssertionJwk: (value: string) => void;
  clientError: string | null;
  onCreateClient: () => void;
  onClearClient: () => void;
  attachDialogOpen: boolean;
  setAttachDialogOpen: (open: boolean) => void;
  attachClientId: string;
  setAttachClientId: (value: string) => void;
  attachError: string | null;
  onAttachClient: () => void;
  onClearAttach: () => void;
  secretResult: SecretResult | null;
  secretDialogOpen: boolean;
  setSecretDialogOpen: (open: boolean) => void;
}) {
  return (
    <>
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
              <textarea
                id="create-service-account-description"
                className="flex min-h-24 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
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
                  <Loader2 className="size-4 animate-spin" />
                  Creation...
                </>
              ) : (
                'Creer'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={clientDialogOpen}
        onOpenChange={(open) => {
          setClientDialogOpen(open);
          if (!open) onClearClient();
        }}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Creer un client OAuth</DialogTitle>
            <DialogDescription>
              Generation d&apos;un client attache au service account selectionne. Le secret est
              affiche une seule fois apres creation.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="client-name">Nom</Label>
              <Input
                id="client-name"
                value={clientName}
                onChange={(event) => setClientName(event.target.value)}
                placeholder="drive-automation"
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="client-scopes">Scopes autorises</Label>
              <textarea
                id="client-scopes"
                className="flex min-h-24 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                value={clientScopes}
                onChange={(event) => setClientScopes(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Separer les scopes par virgule ou nouvelle ligne. Le preset Drive est deja
                pre-rempli.
              </p>
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="client-audiences">Audiences</Label>
                <textarea
                  id="client-audiences"
                  className="flex min-h-20 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  value={clientAudiences}
                  onChange={(event) => setClientAudiences(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="client-resources">Resources</Label>
                <textarea
                  id="client-resources"
                  className="flex min-h-20 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  value={clientResources}
                  onChange={(event) => setClientResources(event.target.value)}
                />
              </div>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="client-required-acr">Required ACR</Label>
              <Input
                id="client-required-acr"
                value={clientRequiredAcr}
                onChange={(event) => setClientRequiredAcr(event.target.value)}
                placeholder="aal2"
              />
            </div>
            <div className="flex items-center gap-2">
              <input
                id="client-assertion-required"
                type="checkbox"
                className="size-4 rounded border-border text-primary focus:ring-ring"
                checked={clientAssertionRequired}
                onChange={(event) => setClientAssertionRequired(event.target.checked)}
              />
              <Label htmlFor="client-assertion-required">Client assertion requise</Label>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="client-jwk">JWK public optionnel</Label>
              <textarea
                id="client-jwk"
                className="flex min-h-28 w-full rounded-lg border border-input bg-transparent px-3 py-2 font-mono text-xs shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                value={clientAssertionJwk}
                onChange={(event) => setClientAssertionJwk(event.target.value)}
                placeholder='{"kty":"RSA",...}'
              />
            </div>
            <DialogError message={clientError} />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setClientDialogOpen(false);
                onClearClient();
              }}
              disabled={busyAction === 'create-client'}
            >
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
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
            <DialogDescription>
              Rattache un client deja cree a ce service account.
            </DialogDescription>
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

      <SecretDialog
        result={secretResult}
        open={secretDialogOpen}
        onOpenChange={setSecretDialogOpen}
      />
    </>
  );
}
