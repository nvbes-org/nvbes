import { accountClient, type OAuthClient } from '@nvbes/identity-client';

export type LinkedApp = OAuthClient;

export function listLinkedApps(signal?: AbortSignal): Promise<LinkedApp[]> {
  return accountClient.listOAuthClients({ signal });
}

export function revokeLinkedApp(clientId: string): Promise<void> {
  return accountClient.revokeOAuthClient(clientId);
}
