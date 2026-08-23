import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

const { assignMock, postMock } = vi.hoisted(() => ({
  assignMock: vi.fn(),
  postMock: vi.fn(),
}));

vi.mock('./identity.http', () => ({
  identityHttpClient: {
    post: postMock,
  },
}));

import { authorizeIdentitySession } from './identity.oauth';

function installTestWindow(search = '') {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: {
        assign: assignMock,
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
    expect(
      Object.keys(postMock.mock.calls[0][3].headers).map((name) => name.toLowerCase()),
    ).not.toContain('authorization');
    expect(assignMock).toHaveBeenCalledWith(
      expect.stringContaining(
        '/oauth/authorize?response_type=code&client_id=cloud-web&request_uri=',
      ),
    );
  });
});
