export function parseAccountCallback(url: URL, expectedState: string | undefined) {
  const params = url.searchParams;
  for (const key of params.keys())
    if (params.getAll(key).length !== 1) throw new Error('Duplicate OAuth response parameter');
  const state = params.get('state');
  if (!expectedState || state !== expectedState || url.hash) throw new Error('Invalid OAuth state');
  const code = params.get('code');
  if (params.has('error')) {
    if (code || params.get('error') !== 'access_denied')
      throw new Error('OAuth authorization failed');
    return { kind: 'denied' as const };
  }
  if (!code || code.length > 4096) throw new Error('Missing OAuth code');
  return { kind: 'code' as const, code, state };
}
