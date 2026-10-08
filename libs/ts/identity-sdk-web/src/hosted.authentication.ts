import type { HostedInteraction } from './hosted.client';
import { hostedRequest, text, type HostedTransport } from './hosted.transport';

export type HostedAuthenticationPolicy = 'primary' | 'recent_mfa' | 'recent_webauthn';

/** Display state only: consent always revalidates policy and proof on the server. */
export interface HostedAuthenticationStatus {
  minimumAuthentication: HostedAuthenticationPolicy;
  needsLogin: boolean;
  needsStepUp: boolean;
  proofExpiresAt: string | null;
}

export async function getHostedAuthenticationStatus(
  config: HostedTransport,
  interaction: HostedInteraction,
): Promise<HostedAuthenticationStatus> {
  const result = await hostedRequest(
    config,
    '/oauth/authorize/authentication',
    {
      interaction: text(interaction.interaction),
    },
    interaction.csrfToken,
  );
  const policy = result.minimum_authentication;
  const login = result.needs_login;
  const stepUp = result.needs_step_up;
  if (
    (policy !== 'primary' && policy !== 'recent_mfa' && policy !== 'recent_webauthn') ||
    typeof login !== 'boolean' ||
    typeof stepUp !== 'boolean' ||
    (login && stepUp) ||
    (policy === 'primary' && stepUp)
  )
    throw new Error('Invalid Identity authentication status.');
  const expires = result.proof_expires_at === null ? null : text(result.proof_expires_at, 64);
  if (
    (expires !== null &&
      (!/^\d{4}-\d{2}-\d{2}T/.test(expires) || !Number.isFinite(Date.parse(expires)))) ||
    (login || stepUp || policy === 'primary' ? expires !== null : expires === null)
  )
    throw new Error('Invalid Identity proof expiry.');
  return {
    minimumAuthentication: policy,
    needsLogin: login,
    needsStepUp: stepUp,
    proofExpiresAt: expires,
  };
}
