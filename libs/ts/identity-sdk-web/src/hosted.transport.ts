export interface HostedTransport {
  baseUrl: string;
  fetchImpl?: typeof fetch;
}

export class HostedIdentityError extends Error {
  constructor(public readonly status: number) {
    super(`Identity hosted request failed (${status}).`);
    this.name = 'HostedIdentityError';
  }
}

export function hostedOrigin(baseUrl: string): string {
  const url = new URL(baseUrl);
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if (
    (url.protocol !== 'https:' && !local) ||
    url.username ||
    url.password ||
    url.pathname !== '/' ||
    url.search ||
    url.hash ||
    typeof location === 'undefined' ||
    location.origin !== url.origin
  ) {
    throw new Error('Hosted Identity operations require the Identity browser origin.');
  }
  return url.origin;
}

export function record(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error('Invalid hosted Identity response.');
  return value as Record<string, unknown>;
}

export function text(value: unknown, max = 1024): string {
  if (
    typeof value !== 'string' ||
    !value ||
    value.length > max ||
    Array.from(value).some(
      (character) => character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127,
    )
  )
    throw new Error('Invalid hosted Identity field.');
  return value;
}

export async function hostedRequest(
  config: HostedTransport,
  path: string,
  body?: object,
  csrf?: string,
): Promise<Record<string, unknown>> {
  return record(await hostedJsonRequest(config, path, body, csrf));
}

/** Bounded JSON transport; each caller validates its endpoint's response shape. */
export async function hostedJsonRequest(
  config: HostedTransport,
  path: string,
  body?: object,
  csrf?: string,
): Promise<unknown> {
  const origin = hostedOrigin(config.baseUrl);
  const headers = new Headers({ Accept: 'application/json' });
  if (body) {
    headers.set('Content-Type', 'application/json');
    headers.set('X-CSRF-Token', text(csrf));
  }
  const response = await (config.fetchImpl ?? fetch)(`${origin}${path}`, {
    method: body ? 'POST' : 'GET',
    headers,
    credentials: 'same-origin',
    redirect: 'error',
    cache: 'no-store',
    signal: AbortSignal.timeout(10_000),
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  if (!response.ok) {
    await response.body?.cancel();
    throw new HostedIdentityError(response.status);
  }
  if (
    response.headers.get('content-type')?.split(';')[0]?.trim().toLowerCase() !==
      'application/json' ||
    Number(response.headers.get('content-length')) > 65_536 ||
    !response.body
  ) {
    await response.body?.cancel();
    throw new Error('Invalid hosted Identity response.');
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > 65_536) throw new Error('Hosted Identity response is too large.');
      chunks.push(value);
    }
  } finally {
    await reader.cancel();
    reader.releaseLock();
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.length;
  }
  try {
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new Error('Invalid hosted Identity response.');
  }
}
