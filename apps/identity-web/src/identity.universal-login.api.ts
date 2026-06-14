import { z } from 'zod';

import { identityHttpClient } from './identity.http';

const HostedClientSchema = z.object({
  client_id: z.string(),
  name: z.string(),
});

export const HostedLoginDecisionSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('login_required'),
    login_url: z.string(),
    state_id: z.string(),
  }),
  z.object({
    kind: z.literal('consent_required'),
    state_id: z.string(),
    client: HostedClientSchema,
    scope: z.string(),
  }),
  z.object({
    kind: z.literal('redirect'),
    redirect_url: z.string(),
  }),
  z.object({
    kind: z.literal('error_page'),
    code: z.string(),
    message: z.string(),
  }),
]);

export type HostedLoginDecision = z.infer<typeof HostedLoginDecisionSchema>;

export function readHostedStateId(searchParams: URLSearchParams): string | null {
  return searchParams.get('state_id');
}

export async function getHostedLoginDecision(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.get(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}`,
    HostedLoginDecisionSchema,
  );
}

export async function authorizeHostedLogin(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/authorize`,
    HostedLoginDecisionSchema,
    {},
  );
}

export async function approveHostedConsent(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/consent`,
    HostedLoginDecisionSchema,
    { consent_action: 'approve' },
  );
}

export async function denyHostedConsent(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/consent`,
    HostedLoginDecisionSchema,
    { consent_action: 'deny' },
  );
}
