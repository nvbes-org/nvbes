import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { ServiceAccountClient } from '../identity.service-accounts.api';

export function OAuthClientRow({
  client,
  onRotateClientSecret,
  onRevokeClient,
}: {
  client: ServiceAccountClient;
  onRotateClientSecret: (client: ServiceAccountClient) => void;
  onRevokeClient: (client: ServiceAccountClient) => void;
}) {
  return (
    <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-muted/10 p-4 xl:flex-row xl:items-start xl:justify-between">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <p className="truncate text-sm font-medium">{client.name}</p>
          <Badge variant={client.revoked_at ? 'destructive' : 'secondary'}>
            {client.revoked_at ? 'Revoque' : 'Actif'}
          </Badge>
          <Badge variant="outline">{client.client_id}</Badge>
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          <Badge variant="secondary">
            {client.client_assertion_required
              ? 'client assertion requise'
              : 'assertion optionnelle'}
          </Badge>
          <Badge variant="outline">
            {client.client_assertion_public_key_configured ? 'JWK configure' : 'pas de JWK'}
          </Badge>
          <Badge variant="outline">{client.required_acr}</Badge>
        </div>
        <div className="mt-3 flex flex-wrap gap-1.5">
          {client.allowed_scopes.map((scope) => (
            <Badge key={scope} variant="ghost" className="h-6 rounded-full">
              {scope}
            </Badge>
          ))}
        </div>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => onRotateClientSecret(client)}>
          Rotation
        </Button>
        <Button variant="destructive" onClick={() => onRevokeClient(client)}>
          Revoquer
        </Button>
      </div>
    </div>
  );
}
