import type { IncomingMessage } from 'node:http';

/** The dev proxy is deliberately limited to an explicitly selected loopback backend. */
export function identityBackend(value: string | undefined): string | undefined {
  if (!value) return undefined;
  const url = new URL(value);
  if (
    url.protocol !== 'http:' ||
    url.hostname !== '127.0.0.1' ||
    !url.port ||
    url.pathname !== '/' ||
    url.search ||
    url.hash ||
    url.username ||
    url.password
  ) {
    throw new Error(
      'IDENTITY_WEB_BACKEND_URL must be an HTTP origin on 127.0.0.1 with an explicit port.',
    );
  }
  return url.origin;
}

export function hostedDocument(request: Pick<IncomingMessage, 'method' | 'url' | 'headers'>) {
  if (request.method !== 'GET' || !request.headers.accept?.includes('text/html')) return false;
  const url = new URL(request.url ?? '/', 'http://localhost');
  // Silent authorization is a protocol response, never an interactive screen.
  return (
    url.pathname === '/oauth/authorize' &&
    !url.searchParams.getAll('prompt').some((value) => value.split(' ').includes('none'))
  );
}
