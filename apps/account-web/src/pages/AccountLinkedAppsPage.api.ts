import { identityClient, type OAuthClient } from '@nvbes/identity-client';

export type LinkedApp = OAuthClient;

export function listLinkedApps(signal?: AbortSignal): Promise<LinkedApp[]> {
  return identityClient.listOAuthClients({ signal });
}

export function revokeLinkedApp(clientId: string): Promise<void> {
  return identityClient.revokeOAuthClient(clientId);
}
