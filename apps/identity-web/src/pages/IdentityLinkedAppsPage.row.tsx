import { ExternalLink, X } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item';
import type { LinkedApp } from '@/pages/IdentityLinkedAppsPage.api';
import { formatLinkedAppDate } from './IdentityLinkedAppsPage.utils';

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
    <Item className="gap-3 px-0 py-1">
      <ItemMedia variant="icon" className="size-8 rounded-lg bg-muted">
        <ExternalLink className="size-4 text-muted-foreground" />
      </ItemMedia>
      <ItemContent className="min-w-0 gap-0">
        <ItemTitle className="truncate">{client.name}</ItemTitle>
        <ItemDescription className="text-xs">
          {client.client_type} · Cree le {formatLinkedAppDate(client.created_at)}
        </ItemDescription>
      </ItemContent>
      <ItemActions className="shrink-0">
        <Badge variant="secondary">{client.client_type}</Badge>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
          onClick={() => onRevoke(client.id)}
          disabled={revoking === client.id}
          aria-label={`Revoquer ${client.name}`}
        >
          {revoking === client.id ? <Spinner className="size-3" /> : <X className="size-3.5" />}
        </Button>
      </ItemActions>
    </Item>
  );
}
