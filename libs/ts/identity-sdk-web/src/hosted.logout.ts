import { hostedJsonRequest, record, type HostedTransport } from './hosted.transport';

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
