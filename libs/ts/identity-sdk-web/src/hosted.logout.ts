import {
  hostedJsonRequest,
  hostedRequest,
  record,
  text,
  type HostedTransport,
} from './hosted.transport';

export async function prepareHostedRpLogout(
  config: HostedTransport,
  csrf: string,
  request: string,
): Promise<void> {
  const result = await hostedRequest(
    config,
    '/oauth/logout/prepare',
    { request: text(request, 16_384) },
    csrf,
  );
  if (result.prepared !== true) throw new Error('Invalid logout preparation');
}

export async function confirmHostedRpLogout(
  config: HostedTransport,
  csrf: string,
  request: string,
): Promise<string | null> {
  const result = await hostedRequest(
    config,
    '/oauth/logout/confirm',
    { request: text(request, 16_384) },
    csrf,
  );
  if (result.logged_out !== true) throw new Error('Logout not confirmed');
  if (result.redirect_uri === null) return null;
  const uri = text(result.redirect_uri, 8192);
  const url = new URL(uri);
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if ((!local && url.protocol !== 'https:') || url.username || url.password || url.hash)
    throw new Error('Invalid logout return');
  return uri;
}

/** Read-only, Identity-origin context. Loading this never revokes a session. */
export async function loadHostedLogoutContext(config: HostedTransport): Promise<string | null> {
  const response = record(
    await hostedJsonRequest(config, '/oauth/session/logout-context', undefined, undefined, {
      'X-Nvbes-Session-Context': '1',
    }),
  );
  const csrf = response.session_csrf_token;
  if (csrf === null) return null;
  if (typeof csrf !== 'string' || !/^[A-Za-z0-9_-]{43}$/u.test(csrf))
    throw new Error('Invalid Identity logout context');
  return csrf;
}
