import assert from 'node:assert/strict';
import { createHash, randomBytes } from 'node:crypto';

// Explicit cookie jar: browser cookie policies have their own Playwright tests.
export function oauthClient({ origins, clientId, redirect, email, password }) {
  const cookies = new Map();
  const secret = () => randomBytes(32).toString('base64url');
  let sessionCsrf;
  const request = async (path, options = {}) => {
    const response = await fetch(`${origins.identity}${path}`, {
      ...options,
      redirect: 'manual',
      signal: AbortSignal.timeout(5000),
      headers: {
        cookie: [...cookies].map(([key, value]) => `${key}=${value}`).join('; '),
        ...options.headers,
      },
    });
    for (const cookie of response.headers.getSetCookie()) {
      const pair = cookie.split(';')[0];
      const separator = pair.indexOf('=');
      const name = pair.slice(0, separator);
      const value = pair.slice(separator + 1);
      if (value) cookies.set(name, value);
      else cookies.delete(name);
    }
    return response;
  };
  const post = (path, body, csrf) =>
    request(path, {
      method: 'POST',
      headers: {
        origin: origins.identity,
        'content-type': 'application/json',
        'x-csrf-token': csrf,
      },
      body: JSON.stringify(body),
    });
  const issue = async (service, needsLogin, scope = `${service}:read`, dpopKey) => {
    const verifier = secret();
    const state = secret();
    const query = new URLSearchParams({
      client_id: clientId,
      redirect_uri: redirect,
      response_type: 'code',
      scope: `openid ${scope}`,
      resource: origins[service],
      state,
      nonce: secret(),
      code_challenge: createHash('sha256').update(verifier).digest('base64url'),
      code_challenge_method: 'S256',
    });
    if (dpopKey) query.set('dpop_jkt', dpopKey.jkt);
    const authorization = await request(`/oauth/authorize?${query}`);
    assert.equal(authorization.status, 200, `${service} authorization`);
    const interaction = await authorization.json();
    assert.equal(interaction.needs_login, needsLogin);
    let csrf = interaction.csrf_token;
    if (needsLogin) {
      const login = await post(
        '/oauth/authorize/login',
        { interaction: interaction.interaction, email, password },
        csrf,
      );
      assert.equal(login.status, 200, 'hosted password login');
      const authenticated = await login.json();
      csrf = authenticated.csrf_token;
      sessionCsrf = authenticated.session_csrf_token;
    }
    const approval = await post(
      '/oauth/authorize/approve',
      { interaction: interaction.interaction },
      csrf,
    );
    assert.equal(approval.status, 303, 'consent redirect');
    const callback = new URL(approval.headers.get('location'));
    assert.equal(callback.origin, origins.account);
    assert.equal(callback.searchParams.get('state'), state);
    const exchange = await request('/oauth/token', {
      method: 'POST',
      headers: dpopKey ? { dpop: dpopKey.proof('POST', `${origins.identity}/oauth/token`) } : {},
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code: callback.searchParams.get('code'),
        client_id: clientId,
        redirect_uri: redirect,
        code_verifier: verifier,
      }),
    });
    assert.equal(exchange.status, 200, `${service} token exchange`);
    const tokens = await exchange.json();
    assert.equal(tokens.token_type, dpopKey ? 'DPoP' : 'Bearer');
    assert.equal(typeof tokens.access_token, 'string');
    return tokens.access_token;
  };
  return { issue, logout: () => post('/oauth/logout', {}, sessionCsrf) };
}
