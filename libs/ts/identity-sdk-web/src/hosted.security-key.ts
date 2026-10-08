import { loginHostedPassword, logoutHostedSession, type HostedInteraction } from './hosted.client';
import { stepUpHostedPasskey } from './hosted.webauthn';
import type { HostedTransport } from './hosted.transport';
import type { WebauthnGetOptions } from './webauthn.types';

export interface HostedSecurityKeyLogin {
  interaction: HostedInteraction;
  stepUpExpiresAt: string;
}

export class HostedSecurityKeyLoginError extends Error {
  constructor(public readonly sessionRevocationConfirmed: boolean) {
    super(
      sessionRevocationConfirmed
        ? 'Security key login failed; the password session was revoked.'
        : 'Security key login failed; session revocation could not be confirmed.',
    );
    this.name = 'HostedSecurityKeyLoginError';
  }
}

/** Password first keeps credential identifiers behind authentication.
 * Returns only after a user-verified assertion; failure attempts session logout.
 * This client flow does not replace server-side mandatory-MFA authorization policy.
 */
export async function loginHostedSecurityKey(
  config: HostedTransport,
  interaction: HostedInteraction,
  credentials: { email: string; password: string },
  browserOptions?: WebauthnGetOptions,
): Promise<HostedSecurityKeyLogin> {
  const authenticated = await loginHostedPassword(config, interaction, credentials);
  // loginHostedPassword validates this value before returning.
  const csrf = authenticated.sessionCsrfToken;
  if (!csrf) throw new Error('Identity did not provide session CSRF.');
  try {
    const stepUpExpiresAt = await stepUpHostedPasskey(config, csrf, browserOptions);
    if (Date.parse(stepUpExpiresAt) <= Date.now())
      throw new Error('Identity returned an expired security key step-up.');
    return { interaction: authenticated, stepUpExpiresAt };
  } catch {
    let revoked = false;
    try {
      await logoutHostedSession(config, csrf);
      revoked = true;
    } catch {
      // A lost logout response does not prove that the server ended the session.
    }
    throw new HostedSecurityKeyLoginError(revoked);
  }
}
