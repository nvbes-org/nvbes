import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

const { getMock, postMock } = vi.hoisted(() => ({
  getMock: vi.fn(),
  postMock: vi.fn(),
}));

vi.mock('./identity.http', () => ({
  identityHttpClient: {
    post: postMock,
    get: getMock,
  },
}));

import { authorizeIdentitySession } from './identity.oauth';

function installTestWindow(search = '') {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: {
        assign: vi.fn(),
        origin: 'http://localhost:3001',
        search,
      },
      sessionStorage: {
        getItem: vi.fn(),
        setItem: vi.fn(),
        removeItem: vi.fn(),
      },
    },
    writable: true,
  });
}

afterEach(() => {
  vi.resetAllMocks();
  Reflect.deleteProperty(globalThis, 'window');
});

describe('authorizeIdentitySession', () => {
  it('never sends a stale bearer header for browser oauth approval', async () => {
    installTestWindow();
    postMock.mockResolvedValue({
      request_uri: 'urn:ietf:params:oauth:request_uri:gxpar_test',
      expires_in: 90,
    });
    getMock.mockResolvedValue({
      code: 'oauth-code',
      redirect_uri: 'http://localhost:5173/callback',
      state: 'oauth-state',
    });

    await authorizeIdentitySession('stale-session-token', {
      clientId: 'cloud-web',
      redirectUri: 'http://localhost:5173/callback',
      scope: 'openid profile email offline_access drive:read drive:write',
      state: 'oauth-state',
      codeChallenge: 'challenge',
      codeChallengeMethod: 'S256',
      nonce: 'oidc-nonce',
      consentAction: 'approve',
    });

    expect(postMock).toHaveBeenCalledWith(
      '/oauth/par',
      expect.anything(),
      expect.any(URLSearchParams),
      expect.objectContaining({
        headers: expect.not.objectContaining({
          Authorization: expect.any(String),
        }),
      }),
    );
    const parBody = postMock.mock.calls[0][2];
    expect(parBody.get('nonce')).toBe('oidc-nonce');
    expect(getMock).toHaveBeenCalledWith(
      expect.stringContaining('/oauth/authorize?'),
      expect.anything(),
      expect.objectContaining({
        headers: expect.not.objectContaining({
          Authorization: expect.any(String),
        }),
      }),
    );

    for (const options of [postMock.mock.calls[0][3], getMock.mock.calls[0][2]]) {
      expect(Object.keys(options.headers).map((name) => name.toLowerCase())).not.toContain(
        'authorization',
      );
    }
  });
});
