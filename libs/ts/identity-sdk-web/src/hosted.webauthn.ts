import { hostedRequest, text, type HostedTransport } from './hosted.transport';
import type { HostedInteraction } from './hosted.client';
import { hostedCreationOptions, hostedRequestOptions } from './hosted.webauthn.options';
import { createWebAuthnCredential, getWebAuthnCredential } from './webauthn.credentials';
import { serializeCredential } from './webauthn.codec';
import type { WebauthnCreateOptions, WebauthnGetOptions } from './webauthn.types';

export async function registerHostedPasskey(
  config: HostedTransport,
  sessionCsrf: string,
  label: string,
  browserOptions?: WebauthnCreateOptions,
): Promise<string> {
  const checkedLabel = text(label, 256);
  // Match Rust's char count (Unicode scalar values), not grapheme clusters.
  if (Array.from(checkedLabel).length > 128 || /\p{Cc}/u.test(checkedLabel))
    throw new Error('Invalid Identity credential label.');
  const start = await hostedRequest(
    config,
    '/oauth/session/webauthn/registration/options',
    {},
    sessionCsrf,
  );
  const ceremonyId = text(start.ceremony_id, 64);
  const credential = await createWebAuthnCredential(
    hostedCreationOptions(start.options),
    browserOptions,
  );
  const result = await hostedRequest(
    config,
    '/oauth/session/webauthn/registration/finish',
    {
      ceremony_id: ceremonyId,
      credential: serializeCredential(credential),
      label: checkedLabel,
    },
    sessionCsrf,
  );
  return text(result.credential_id, 64);
}

export async function loginHostedPasskey(
  config: HostedTransport,
  interaction: HostedInteraction,
  browserOptions?: WebauthnGetOptions,
): Promise<HostedInteraction> {
  const start = await hostedRequest(
    config,
    '/oauth/authorize/passkey/options',
    {
      interaction: text(interaction.interaction),
    },
    interaction.csrfToken,
  );
  const ceremonyId = text(start.ceremony_id, 64);
  const credential = await getWebAuthnCredential(
    hostedRequestOptions(start.options),
    browserOptions,
  );
  const result = await hostedRequest(
    config,
    '/oauth/authorize/passkey/finish',
    {
      interaction: interaction.interaction,
      ceremony_id: ceremonyId,
      credential: serializeCredential(credential),
    },
    interaction.csrfToken,
  );
  if (result.interaction !== interaction.interaction)
    throw new Error('Hosted interaction mismatch.');
  return {
    ...interaction,
    needsLogin: false,
    csrfToken: text(result.csrf_token),
    sessionCsrfToken: text(result.session_csrf_token),
  };
}

export async function stepUpHostedPasskey(
  config: HostedTransport,
  sessionCsrf: string,
  browserOptions?: WebauthnGetOptions,
): Promise<string> {
  const start = await hostedRequest(
    config,
    '/oauth/session/webauthn/step-up/options',
    {},
    sessionCsrf,
  );
  const ceremonyId = text(start.ceremony_id, 64);
  const credential = await getWebAuthnCredential(
    hostedRequestOptions(start.options),
    browserOptions,
  );
  const result = await hostedRequest(
    config,
    '/oauth/session/webauthn/step-up/finish',
    {
      ceremony_id: ceremonyId,
      credential: serializeCredential(credential),
    },
    sessionCsrf,
  );
  const expires = text(result.expires_at, 64);
  if (result.step_up !== true || !Number.isFinite(Date.parse(expires)))
    throw new Error('Identity did not confirm step-up.');
  return expires;
}
