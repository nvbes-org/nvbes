import { Copy } from 'lucide-react';
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
import { formatDateTime, type SecretResult } from './WorkspaceServiceAccounts.helpers';

export function DialogError({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

export function SecretDialog({
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
              <div className="mt-4 break-all rounded-xl border border-border/70 bg-background p-3 font-mono text-sm">
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
