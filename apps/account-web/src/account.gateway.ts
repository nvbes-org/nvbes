import {
  dpopFetch,
  BrowserSessionStorage,
  createAuthorizationRequest,
  discardAuthorizationRequest,
  exchangeAuthorizationCode,
  type AuthorizationCodeTokenResponse,
} from '@nvbes/identity-sdk-web/oauth';
import { loadAccountConfig, type AccountConfig } from './account.config';
import { parseAccountCallback } from './account.callback';
import { accountTransport } from './account.transport';
import { parseAccountProfile, type AccountProfile } from './account.profile';

export interface AccountGateway {
  initialize(): Promise<void>;
  authorize(): Promise<string>;
  callback(url: URL): Promise<'connected' | 'denied'>;
  profile(): Promise<AccountProfile>;
  expiration(): number;
  clear(): void;
}

export function accountGateway(siteOrigin: string): AccountGateway {
  let config: AccountConfig | undefined;
  let tokens: AuthorizationCodeTokenResponse | undefined;
  let expiresAt = 0;
  let disposed = false;
  const lifetime = new AbortController();
  const storage = () =>
    new BrowserSessionStorage(sessionStorage, 'nvbes.account.oauth.transaction');
  const context = () => {
    if (!config || disposed) throw new Error('Account unavailable');
    return {
      baseUrl: config.identityOrigin,
      clientId: config.clientId,
      redirectUri: `${siteOrigin}/oauth/callback`,
      resource: config.accountApiOrigin,
      storage: storage(),
      fetchImpl: accountTransport([config.identityOrigin], lifetime.signal),
    };
  };
  return {
    expiration: () => expiresAt,
    async initialize() {
      config = await loadAccountConfig(
        AbortSignal.any([lifetime.signal, AbortSignal.timeout(10_000)]),
      );
    },
    async authorize() {
      const options = context();
      await discardAuthorizationRequest({ storage: options.storage });
      const request = await createAuthorizationRequest(options, { scope: 'openid account:read' });
      if (disposed) {
        await discardAuthorizationRequest({ storage: options.storage });
        throw new Error('Account closed');
      }
      return request.authorizationUrl;
    },
    async callback(url) {
      const options = context();
      try {
        const result = parseAccountCallback(url, options.storage.getTransaction()?.state);
        if (result.kind === 'denied') return 'denied';
        const started = Date.now();
        const exchanged = await exchangeAuthorizationCode(options, result);
        if (
          disposed ||
          !exchanged.dpopKey ||
          !exchanged.identity ||
          exchanged.tokenType !== 'DPoP' ||
          exchanged.scope.split(' ').some((scope) => !['openid', 'account:read'].includes(scope)) ||
          !exchanged.scope.split(' ').includes('account:read')
        )
          throw new Error('Invalid Account grant');
        tokens = exchanged;
        expiresAt = started + exchanged.expiresIn * 1000;
        return 'connected';
      } finally {
        // A failed exchange may already have consumed the code: never automatically retry it.
        await discardAuthorizationRequest({ storage: options.storage });
      }
    },
    async profile() {
      const options = context();
      const current = tokens;
      if (!current?.identity || !current.dpopKey || Date.now() >= expiresAt)
        throw new Error('Account session expired');
      const response = await dpopFetch(`${options.resource}/api/v1/profile`, {
        accessToken: current.accessToken,
        key: current.dpopKey,
        signal: AbortSignal.any([lifetime.signal, AbortSignal.timeout(10_000)]),
        cache: 'no-store',
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) throw new Error('Account profile unavailable');
      return parseAccountProfile(await response.json(), current.identity.subject);
    },
    clear() {
      disposed = true;
      tokens = undefined;
      expiresAt = 0;
      lifetime.abort();
    },
  };
}
