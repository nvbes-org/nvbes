import { expect, it, vi } from 'vite-plus/test';
import { MemoryStorage } from './storage';
import { createAuthorizationRequest } from './oauth.authorization-request';
import { exchangeAuthorizationCode } from './oauth.authorization-code';

function setup(payload: unknown, status = 200) {
  const storage = new MemoryStorage();
  storage.saveTransaction({
    state: 'state',
    codeVerifier: 'verifier',
    nonce: null,
    createdAt: 1000,
    returnTo: '/profile',
  });
  const fetchImpl = vi.fn().mockResolvedValue(Response.json(payload, { status }));
  return {
    baseUrl: 'https://identity.test/',
    clientId: 'web',
    redirectUri: 'https://app.test/callback',
    storage,
    fetchImpl,
    now: () => 1000,
  };
}
const tokens = { access_token: 'access', token_type: 'Bearer', scope: 'profile', expires_in: 300 };

it.each([
  null,
  [],
  'text',
  {},
  { request_uri: 1, expires_in: 30 },
  { request_uri: ' ', expires_in: 30 },
  { request_uri: 'uri', expires_in: '30' },
  { request_uri: 'uri', expires_in: 0 },
  { request_uri: 'uri', expires_in: -1 },
])('rejects malformed PAR response %j and clears the transaction', async (payload) => {
  const config = setup(payload);
  await expect(createAuthorizationRequest(config)).rejects.toThrow(
    'invalid pushed authorization response',
  );
  expect(config.storage.getTransaction()).toBeNull();
});
it.each([null, [], 'text'])('rejects non-object token response %j', async (payload) => {
  await expect(
    exchangeAuthorizationCode(setup(payload), { code: 'code', state: 'state' }),
  ).rejects.toThrow('invalid OAuth token response');
});
it.each(['access_token', 'token_type', 'scope'])(
  'requires a nonblank string for %s',
  async (field) => {
    for (const value of [null, 7, '']) {
      await expect(
        exchangeAuthorizationCode(setup({ ...tokens, [field]: value }), {
          code: 'code',
          state: 'state',
        }),
      ).rejects.toThrow(`missing ${field}`);
    }
  },
);
it.each([null, '300', 0, -1])('rejects invalid token lifetime %j', async (expires_in) => {
  await expect(
    exchangeAuthorizationCode(setup({ ...tokens, expires_in }), { code: 'code', state: 'state' }),
  ).rejects.toThrow('invalid OAuth token lifetime');
});
it.each([null, '', ' ', 5])('treats optional token fields %j as absent', async (value) => {
  const result = await exchangeAuthorizationCode(
    setup({ ...tokens, refresh_token: value, id_token: value }),
    { code: 'code', state: 'state' },
  );
  expect(result).toMatchObject({ refreshToken: null, idToken: null, returnTo: '/profile' });
});
it.each([
  [{ error_description: 'description', message: 'message', error: 'error' }, 'description'],
  [{ error_description: ' ', message: 'message', error: 'error' }, 'message'],
  [{ message: 7, error: 'error' }, 'error'],
  [{ error: {} }, 'status 503'],
  [null, 'status 503'],
] as const)(
  'propagates OAuth errors with deterministic field precedence (%j)',
  async (payload, message) => {
    await expect(createAuthorizationRequest(setup(payload, 503))).rejects.toThrow(message);
    await expect(
      exchangeAuthorizationCode(setup(payload, 503), { code: 'code', state: 'state' }),
    ).rejects.toThrow(message);
  },
);
it('enforces transaction time bounds before any network request', async () => {
  for (const now of [999, 901001]) {
    const config = { ...setup(tokens), now: () => now };
    await expect(
      exchangeAuthorizationCode(config, { code: 'code', state: 'state' }),
    ).rejects.toThrow('expired');
    expect(config.fetchImpl).not.toHaveBeenCalled();
    expect(config.storage.getTransaction()).toBeNull();
  }
  const config = { ...setup(tokens), now: () => 901000 };
  await expect(
    exchangeAuthorizationCode(config, { code: 'code', state: 'state' }),
  ).resolves.toMatchObject({ accessToken: 'access' });
});
it.each(['baseUrl', 'clientId', 'redirectUri'] as const)(
  'rejects blank %s before posting authorization',
  async (field) => {
    const config = { ...setup({ request_uri: 'uri', expires_in: 30 }), [field]: ' ' };
    await expect(createAuthorizationRequest(config)).rejects.toThrow('required');
    expect(config.fetchImpl).not.toHaveBeenCalled();
  },
);
it('omits nonce outside openid scope and honors the explicitly selected audience', async () => {
  const config = { ...setup({ request_uri: 'uri', expires_in: 30 }), audience: 'default-api' };
  await createAuthorizationRequest(config, {
    scope: 'profile',
    state: 'chosen-state',
    audience: ' selected-api ',
  });
  const body = config.fetchImpl.mock.calls[0][1].body as URLSearchParams;
  expect(body.has('nonce')).toBe(false);
  expect(body.get('audience')).toBe('selected-api');
  expect(body.get('state')).toBe('chosen-state');
  expect(config.storage.getTransaction()?.nonce).toBeNull();
});
