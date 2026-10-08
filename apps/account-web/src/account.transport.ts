/** OAuth transport: do not add application CSRF/AJAX headers to protocol requests. */
export function accountTransport(origins: readonly string[], signal: AbortSignal): typeof fetch {
  const allowed = new Set(origins);
  return async (input, init) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.origin);
    if (!allowed.has(url.origin) || url.username || url.password || url.hash)
      throw new Error('Account transport rejected the destination');
    return fetch(input, {
      ...init,
      credentials: 'omit',
      redirect: 'error',
      cache: 'no-store',
      signal: AbortSignal.any([
        signal,
        AbortSignal.timeout(10_000),
        ...(init?.signal ? [init.signal] : []),
      ]),
    });
  };
}
