import {
  hostedOrigin,
  hostedRequest,
  record,
  text,
  type HostedTransport,
} from './hosted.transport';
export { HostedIdentityError } from './hosted.transport';
export type { HostedTransport } from './hosted.transport';

export interface HostedInteraction {
  interaction: string;
  csrfToken: string;
  sessionCsrfToken: string | null;
  needsLogin: boolean;
  clientId: string;
  scope: string;
}

/** For an authorization JSON response obtained during top-level navigation. */
export function parseHostedInteraction(payload: unknown): HostedInteraction {
  const body = record(payload);
  if (typeof body.needs_login !== 'boolean') throw new Error('Invalid hosted login state.');
  return {
    interaction: text(body.interaction),
    csrfToken: text(body.csrf_token),
    sessionCsrfToken: body.session_csrf_token === null ? null : text(body.session_csrf_token),
    needsLogin: body.needs_login,
    clientId: text(body.client_id, 128),
    scope: text(body.scope),
  };
}

export async function loadHostedAuthorization(
  config: HostedTransport,
  authorizationUrl: string,
): Promise<HostedInteraction> {
  const url = new URL(authorizationUrl);
  if (
    url.origin !== hostedOrigin(config.baseUrl) ||
    url.pathname !== '/oauth/authorize' ||
    url.username ||
    url.password ||
    url.hash
  )
    throw new Error('Invalid hosted authorization URL.');
  return parseHostedInteraction(await hostedRequest(config, `${url.pathname}${url.search}`));
}

export async function loginHostedPassword(
  config: HostedTransport,
  interaction: HostedInteraction,
  credentials: { email: string; password: string },
): Promise<HostedInteraction> {
  if (
    typeof credentials.password !== 'string' ||
    !credentials.password ||
    credentials.password.length > 1024
  )
    throw new Error('Invalid password input.');
  const result = await hostedRequest(
    config,
    '/oauth/authorize/login',
    {
      interaction: text(interaction.interaction),
      email: text(credentials.email, 320),
      password: credentials.password,
    },
    interaction.csrfToken,
  );
  if (result.interaction !== interaction.interaction)
    throw new Error('Hosted interaction mismatch.');
  return {
    ...interaction,
    csrfToken: text(result.csrf_token),
    sessionCsrfToken: text(result.session_csrf_token),
    needsLogin: false,
  };
}

export async function completeHostedConsent(
  config: HostedTransport,
  interaction: HostedInteraction,
  decision: 'approve' | 'deny',
): Promise<string> {
  if (decision !== 'approve' && decision !== 'deny') throw new Error('Invalid consent decision.');
  const result = await hostedRequest(
    config,
    `/oauth/authorize/${decision}`,
    { interaction: text(interaction.interaction) },
    interaction.csrfToken,
  );
  const destination = text(result.redirect_uri, 8192);
  const url = new URL(destination);
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if ((url.protocol !== 'https:' && !local) || url.username || url.password || url.hash)
    throw new Error('Invalid Identity navigation destination.');
  return destination;
}

export async function logoutHostedSession(
  config: HostedTransport,
  sessionCsrfToken: string,
): Promise<void> {
  const result = await hostedRequest(config, '/oauth/logout', {}, sessionCsrfToken);
  if (result.logged_out !== true) throw new Error('Identity did not confirm logout.');
}
