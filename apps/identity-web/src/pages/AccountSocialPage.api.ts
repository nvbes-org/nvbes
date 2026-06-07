import { z } from 'zod';
import { identityHttpClient } from '../identity.http';

const LinkedIdentitySchema = z.object({
  id: z.string(),
  provider_type: z.string(),
  provider_id: z.string(),
  subject: z.string(),
  created_at: z.string().optional(),
  principal_display_name: z.string().optional(),
});

const LinkedIdentitiesResponseSchema = z.object({
  identities: z.array(LinkedIdentitySchema),
});

export type LinkedIdentity = z.infer<typeof LinkedIdentitySchema>;

export function listLinkedIdentities(
  tenantId: string,
  signal?: AbortSignal,
): Promise<LinkedIdentity[]> {
  return identityHttpClient
    .get(`/tenants/${tenantId}/linked-identities`, LinkedIdentitiesResponseSchema, { signal })
    .then((response) => response.identities)
    .catch(() => []);
}

export function unlinkIdentity(tenantId: string, identityId: string): Promise<void> {
  const emptySchema = z.undefined();
  return identityHttpClient.delete(
    `/tenants/${tenantId}/linked-identities/${identityId}`,
    emptySchema,
  );
}
