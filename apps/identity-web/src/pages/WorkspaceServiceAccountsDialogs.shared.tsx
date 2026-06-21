import { InvisibleUnicodeWarning } from '@nvbes/web-runtime';
import { ClipboardButton } from '@nvbes/web-ui';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
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
    <Alert variant="destructive">
      <AlertDescription>{message}</AlertDescription>
    </Alert>
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
            <Card className="p-4">
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
              <Card className="mt-4 break-all p-3 font-mono text-sm">{result.client_secret}</Card>
              <InvisibleUnicodeWarning value={result.client_secret} />
            </Card>
          </div>
        )}
        <DialogFooter>
          {result ? <ClipboardButton value={result.client_secret} label="Copier" /> : null}
          <Button onClick={() => onOpenChange(false)}>Fermer</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
