import { z } from 'zod';

import { identityHttpClient } from '../identity.http';

export const SecurityEventSchema = z.object({
  id: z.string(),
  event_type: z.string(),
  created_at: z.string(),
  ip_address: z.string().nullable().optional(),
  user_agent: z.string().nullable().optional(),
  status: z.string().optional(),
});

export const SecurityEventsResponseSchema = z.object({
  events: z.array(SecurityEventSchema),
  next_cursor: z.string().nullable().optional(),
});

export type SecurityEvent = z.infer<typeof SecurityEventSchema>;

export function listSecurityEvents(workspaceId: string, limit = 20) {
  return identityHttpClient.get(
    `/workspaces/${workspaceId}/security-events?limit=${limit}`,
    SecurityEventsResponseSchema,
  );
}

export const eventLabels: Record<string, string> = {
  login_success: 'Connexion reussie',
  login_failed: 'Echec de connexion',
  password_changed: 'Mot de passe modifie',
  password_reset: 'Reinitialisation du mot de passe',
  mfa_enrolled: 'MFA activee',
  mfa_removed: 'MFA desactivee',
  session_revoked: 'Session revoquee',
  consent_granted: 'Consentement accorde',
  consent_revoked: 'Consentement revoque',
  account_created: 'Compte cree',
  email_verified: 'Email verifie',
  export_requested: 'Export demande',
  account_deleted: 'Compte supprime',
};
