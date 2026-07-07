import { identityHttpClient } from './identity.http';
import {
  AttachOAuthClientInputSchema,
  CreateServiceAccountInputSchema,
  CreateServiceAccountOAuthClientInputSchema,
  CreateServiceAccountOAuthClientResultSchema,
  RotateOAuthClientSecretResultSchema,
  ServiceAccountsResultSchema,
  ServiceAccountSchema,
  UpdateServiceAccountInputSchema,
  type AttachOAuthClientInput,
  type CreateServiceAccountInput,
  type CreateServiceAccountOAuthClientInput,
  type UpdateServiceAccountInput,
} from './identity.service-accounts.schemas';

export type {
  AttachOAuthClientInput,
  CreateServiceAccountInput,
  CreateServiceAccountOAuthClientInput,
  CreateServiceAccountOAuthClientResult,
  RotateOAuthClientSecretResult,
  ServiceAccount,
  ServiceAccountClient,
  ServiceAccountsResult,
  UpdateServiceAccountInput,
} from './identity.service-accounts.schemas';

type RequestOptions = { signal?: AbortSignal };

export function listServiceAccounts(workspaceId: string, options?: RequestOptions) {
  return identityHttpClient
    .get(`/workspaces/${workspaceId}/service-accounts`, ServiceAccountsResultSchema, options)
    .then((response) => response.service_accounts);
}

export function getServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.get(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}`,
    ServiceAccountSchema,
  );
}

export function createServiceAccount(workspaceId: string, input: CreateServiceAccountInput) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts`,
    ServiceAccountSchema,
    CreateServiceAccountInputSchema.parse(input),
  );
}

export function updateServiceAccount(
  workspaceId: string,
  serviceAccountId: string,
  input: UpdateServiceAccountInput,
) {
  return identityHttpClient.request(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}`,
    ServiceAccountSchema,
    {
      method: 'PATCH',
      body: UpdateServiceAccountInputSchema.parse(input),
    },
  );
}

export function suspendServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/suspend`,
    ServiceAccountSchema,
    {},
  );
}

export function reactivateServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/reactivate`,
    ServiceAccountSchema,
    {},
  );
}

export function createServiceAccountOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  input: CreateServiceAccountOAuthClientInput,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients`,
    CreateServiceAccountOAuthClientResultSchema,
    CreateServiceAccountOAuthClientInputSchema.parse(input),
  );
}

export function attachOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  input: AttachOAuthClientInput,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients:attach`,
    ServiceAccountSchema,
    AttachOAuthClientInputSchema.parse(input),
  );
}

export function revokeServiceAccountOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  clientId: string,
) {
  return identityHttpClient.delete(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients/${clientId}`,
    ServiceAccountSchema,
  );
}

export function rotateServiceAccountOAuthClientSecret(
  workspaceId: string,
  serviceAccountId: string,
  clientId: string,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients/${clientId}/rotate-secret`,
    RotateOAuthClientSecretResultSchema,
    {},
  );
}
