import assert from 'node:assert/strict';
import { createAuthorizationRequest } from '../../../libs/ts/identity-sdk-web/src/oauth.authorization-request.ts';
import { exchangeAuthorizationCode } from '../../../libs/ts/identity-sdk-web/src/oauth.authorization-code.ts';
import { MemoryStorage } from '../../../libs/ts/identity-sdk-web/src/storage.ts';
import { OAuthSession } from '../../../libs/ts/identity-sdk-web/src/oauth.session.ts';
import { dpopFetch } from '../../../libs/ts/identity-sdk-web/src/dpop.ts';

// Node exercises the actual SDK over HTTP. IndexedDB has a separate browser proof.
export async function verifySdkRefresh({ origins, endpoints, browser, clientId, redirect }) {
  const sessions = [];
  for (const service of ['account', 'billing']) {
    const keys = new Map();
    const config = {
      baseUrl: origins.identity,
      clientId,
      redirectUri: redirect,
      resource: origins[service],
      storage: new MemoryStorage(),
      dpopStore: {
        async save(id, key) {
          keys.set(id, key);
        },
        async load(id) {
          return keys.get(id) ?? null;
        },
        async remove(id) {
          keys.delete(id);
        },
      },
    };
    const started = await createAuthorizationRequest(config, {
      scope: `openid offline_access ${service}:read`,
    });
    const callback = await browser.authorize(started.authorizationUrl, false);
    const tokens = await exchangeAuthorizationCode(config, {
      code: callback.searchParams.get('code'),
      state: callback.searchParams.get('state'),
    });
    assert.equal(tokens.tokenType, 'DPoP');
    assert.equal(keys.size, 0);
    const session = new OAuthSession(config, tokens);
    const first = session.refresh();
    const concurrent = session.refresh();
    assert.equal(first, concurrent);
    const rotated = await first;
    assert.notEqual(rotated.refreshToken, tokens.refreshToken);
    assert.equal(rotated.dpopKey.jkt, tokens.dpopKey.jkt);
    const again = await session.refresh();
    assert.notEqual(again.refreshToken, rotated.refreshToken);
    const response = await dpopFetch(endpoints[service], {
      key: again.dpopKey,
      accessToken: again.accessToken,
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 200, `SDK DPoP ${service} after refresh`);
    await response.arrayBuffer();
    sessions.push(session);
  }
  return sessions;
}
