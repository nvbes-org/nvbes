import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { NvbesIdentityWeb } from './identity-web.client';
import { MemoryStorage } from './storage';
import * as mfa from './mfa';
import { createAuthorizationRequest } from './oauth.authorization-request';
import { exchangeAuthorizationCode } from './oauth.authorization-code';

vi.mock('./mfa', () => ({
  listMfaFactors: vi.fn(),
  setupTotp: vi.fn(),
  confirmTotp: vi.fn(),
  startWebAuthnRegistration: vi.fn(),
  finishWebAuthnRegistration: vi.fn(),
  registerWebAuthnCredential: vi.fn(),
  startWebAuthnAuthentication: vi.fn(),
  completeWebAuthnStepUp: vi.fn(),
  generateRecoveryCodes: vi.fn(),
  removeMfaFactor: vi.fn(),
  stepUp: vi.fn(),
}));
vi.mock('./oauth.authorization-request', () => ({ createAuthorizationRequest: vi.fn() }));
vi.mock('./oauth.authorization-code', () => ({ exchangeAuthorizationCode: vi.fn() }));
const config = {
  baseUrl: 'https://identity.test',
  clientId: 'web',
  redirectUri: 'https://app.test/callback',
  audience: 'api',
};
beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
});
afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

it('forwards OAuth configuration and redirects only after creating an authorization request', async () => {
  const storage = new MemoryStorage();
  const client = new NvbesIdentityWeb({ ...config, storage });
  const assign = vi.fn();
  vi.stubGlobal('window', { location: { assign } });
  vi.mocked(createAuthorizationRequest).mockResolvedValue({
    authorizationUrl: 'https://identity.test/oauth/authorize?request_uri=one',
    state: 'state',
  });
  await client.redirectToLogin({ returnTo: '/profile' });
  expect(createAuthorizationRequest).toHaveBeenCalledWith(
    { ...config, storage },
    { returnTo: '/profile' },
  );
  expect(assign).toHaveBeenCalledExactlyOnceWith(
    'https://identity.test/oauth/authorize?request_uri=one',
  );
  await client.createAuthorizationRequest();
  expect(createAuthorizationRequest).toHaveBeenLastCalledWith({ ...config, storage }, {});
  await client.exchangeAuthorizationCode({ code: 'code', state: 'state' });
  expect(exchangeAuthorizationCode).toHaveBeenCalledExactlyOnceWith(
    {
      baseUrl: config.baseUrl,
      clientId: config.clientId,
      redirectUri: config.redirectUri,
      storage,
    },
    { code: 'code', state: 'state' },
  );
  storage.saveTransaction({
    state: 'state',
    codeVerifier: 'verifier',
    nonce: null,
    createdAt: 1,
    returnTo: '/',
  });
  client.clearAuthorizationTransaction();
  expect(storage.getTransaction()).toBeNull();
});

it.each([undefined, '2'])(
  'sends logout with the correct scoped CSRF token (account %s)',
  async (authuser) => {
    vi.stubGlobal('window', {
      location: { pathname: authuser ? `/account/${authuser}/profile` : '/', search: '' },
    });
    vi.stubGlobal('document', { cookie: 'csrf_token=default; csrf_token_2=scoped' });
    const client = new NvbesIdentityWeb(config);
    await expect(client.logout()).resolves.toBeUndefined();
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, options] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe('https://identity.test/auth/logout');
    expect(options).toMatchObject({ method: 'POST', credentials: 'include' });
    const headers = new Headers(options?.headers);
    expect(headers.get('X-Auth-User')).toBe(authuser ?? null);
    expect(headers.get('X-CSRF-Token')).toBe(authuser ? 'scoped' : 'default');
  },
);
it('does not report successful logout for a rejected or failed request', async () => {
  vi.stubGlobal('window', undefined);
  vi.stubGlobal('document', undefined);
  const client = new NvbesIdentityWeb(config);
  for (const status of [401, 403, 500]) {
    vi.mocked(fetch).mockResolvedValue(new Response(null, { status }));
    await expect(client.logout()).rejects.toMatchObject({ name: 'HttpError', status });
  }
  vi.mocked(fetch).mockRejectedValue(new Error('offline'));
  await expect(client.logout()).rejects.toThrow('offline');
  expect(fetch).toHaveBeenCalledTimes(4);
});

it('forwards every MFA operation with explicit arguments and defaults', async () => {
  const client = new NvbesIdentityWeb(config);
  await client.listMfaFactors('token', { limit: 2, cursor: 'next' });
  expect(mfa.listMfaFactors).toHaveBeenLastCalledWith(config.baseUrl, 'token', {
    limit: 2,
    cursor: 'next',
  });
  await client.listMfaFactors();
  expect(mfa.listMfaFactors).toHaveBeenLastCalledWith(config.baseUrl, undefined, {});
  await client.setupTotp('phone', 'token');
  expect(mfa.setupTotp).toHaveBeenCalledWith(config.baseUrl, 'phone', 'token');
  await client.confirmTotp('factor', '123456', 'token');
  expect(mfa.confirmTotp).toHaveBeenCalledWith(config.baseUrl, 'factor', '123456', 'token');
  await client.startWebAuthnRegistration();
  expect(mfa.startWebAuthnRegistration).toHaveBeenCalledWith(
    config.baseUrl,
    undefined,
    'passkey',
    undefined,
  );
  const credential = { id: 'credential' } as PublicKeyCredential;
  await client.finishWebAuthnRegistration('factor', credential, 'token');
  expect(mfa.finishWebAuthnRegistration).toHaveBeenCalledWith(
    config.baseUrl,
    'factor',
    credential,
    'token',
  );
  await client.registerWebAuthnCredential();
  expect(mfa.registerWebAuthnCredential).toHaveBeenCalledWith(
    config.baseUrl,
    undefined,
    'passkey',
    undefined,
  );
  await client.startWebAuthnAuthentication('token');
  expect(mfa.startWebAuthnAuthentication).toHaveBeenCalledWith(config.baseUrl, 'token');
  await client.completeWebAuthnStepUp('token');
  expect(mfa.completeWebAuthnStepUp).toHaveBeenCalledWith(config.baseUrl, 'token');
  await client.generateRecoveryCodes('password', 'token');
  expect(mfa.generateRecoveryCodes).toHaveBeenCalledWith(config.baseUrl, 'password', 'token');
  await client.removeMfaFactor('factor', 'token');
  expect(mfa.removeMfaFactor).toHaveBeenCalledWith(config.baseUrl, 'factor', 'token');
  await client.stepUp({ recoveryCode: 'recovery' }, 'token');
  expect(mfa.stepUp).toHaveBeenCalledWith(config.baseUrl, { recoveryCode: 'recovery' }, 'token');
});
