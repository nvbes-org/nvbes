import { ExternalLink, X } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { LinkedApp } from '@/pages/AccountLinkedAppsPage.api';
import { formatLinkedAppDate } from './AccountLinkedAppsPage.utils';

export function LinkedAppRow({
  client,
  revoking,
  onRevoke,
}: {
  client: LinkedApp;
  revoking: string | null;
  onRevoke: (clientId: string) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <ExternalLink className="size-4 text-muted-foreground" />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="truncate text-sm font-medium">{client.name}</span>
          <span className="text-xs text-muted-foreground">
            {client.client_type} · Cree le {formatLinkedAppDate(client.created_at)}
          </span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Badge variant="secondary">{client.client_type}</Badge>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
          onClick={() => onRevoke(client.id)}
          disabled={revoking === client.id}
          aria-label={`Revoquer ${client.name}`}
        >
          {revoking === client.id ? (
            <span className="size-3 animate-spin rounded-full border-2 border-current border-t-transparent" />
          ) : (
            <X className="size-3.5" />
          )}
        </Button>
      </div>
    </div>
  );
}
