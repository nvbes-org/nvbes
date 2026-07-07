import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

const postMock = vi.fn();
const getMock = vi.fn();

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
        origin: 'http://localhost:3001',
        search,
      },
      sessionStorage: {
        getItem: vi.fn(),
        setItem: vi.fn(),
        removeItem: vi.fn(),
      },
      assign: vi.fn(),
    },
    writable: true,
  });
}

afterEach(() => {
  vi.resetAllMocks();
  Reflect.deleteProperty(globalThis, 'window');
});

describe('authorizeIdentitySession', () => {
  it('does not send an Authorization bearer header for browser oauth approval', async () => {
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
    expect(getMock).toHaveBeenCalledWith(
      expect.stringContaining('/oauth/authorize?'),
      expect.anything(),
      expect.objectContaining({
        headers: expect.not.objectContaining({
          Authorization: expect.any(String),
        }),
      }),
    );
  });
});
