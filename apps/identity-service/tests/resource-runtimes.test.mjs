import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, randomBytes, randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import test from 'node:test';
import { command, RuntimeFixture, runtimeEnvironment, unusedPort } from './runtime-processes.mjs';

const secret = () => randomBytes(32).toString('base64url');

test(
  'real Identity tokens, resource audiences, logout and Identity outage',
  { timeout: 120_000 },
  async (t) => {
    const fixture = new RuntimeFixture();
    t.after(() => fixture.close());
    const databases = await fixture.database();
    const origins = {};
    const environments = {};
    const binaries = {
      identity: resolve('target/debug/nvbes-identity-service'),
      account: resolve('target/debug/nvbes-account-service'),
      billing: resolve('apps/billing-service/target/debug/nvbes-billing-service'),
    };
    for (const service of Object.keys(binaries)) {
      // WebAuthn requires a domain RP ID; localhost is its development exception.
      const host = service === 'identity' ? 'localhost' : '127.0.0.1';
      origins[service] = `http://${host}:${await unusedPort()}`;
    }
    const { privateKey, publicKey } = generateKeyPairSync('rsa', {
      modulusLength: 2048,
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
      publicKeyEncoding: { type: 'spki', format: 'pem' },
    });
    const resources = ['account', 'billing'].map((service) => ({
      client_id: `${service}-runtime-test`,
      audience: `nvbes-${service}-service`,
      secret: secret(),
    }));
    const redirect = `${origins.account}/callback`;
    const clients = [
      {
        client_id: 'runtime-test-web',
        display_name: 'Isolated runtime test',
        redirect_uris: [redirect],
        post_logout_redirect_uris: [],
        resources: Object.fromEntries(
          ['account', 'billing'].map((service) => [
            origins[service],
            {
              audience: `nvbes-${service}-service`,
              scopes: [`${service}:read`],
            },
          ]),
        ),
        allow_refresh: false,
        require_dpop: false,
      },
    ];
    for (const service of Object.keys(binaries)) {
      const prefix = `NVBES_${service.toUpperCase()}`;
      environments[service] = {
        ...runtimeEnvironment(),
        [`${prefix}_DATABASE_URL`]: databases[service],
        [`${prefix}_BIND_ADDR`]: `127.0.0.1:${new URL(origins[service]).port}`,
        NVBES_IDENTITY_TOKEN_ISSUER: origins.identity,
        NVBES_IDENTITY_TOKEN_KEY_ID: 'runtime-test-key',
        NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: publicKey,
      };
      const resource = resources.find((item) => item.audience === `nvbes-${service}-service`);
      if (resource)
        Object.assign(environments[service], {
          [`${prefix}_IDENTITY_RESOURCE_CLIENT_ID`]: resource.client_id,
          [`${prefix}_IDENTITY_RESOURCE_SECRET`]: resource.secret,
        });
    }
    Object.assign(environments.identity, {
      NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM: privateKey,
      NVBES_IDENTITY_TOKEN_AUDIENCES: 'nvbes-account-service,nvbes-billing-service',
      NVBES_IDENTITY_BROWSER_ORIGIN: origins.identity,
      NVBES_IDENTITY_OAUTH_CLIENTS_JSON: JSON.stringify(clients),
      NVBES_IDENTITY_RESOURCE_SERVERS_JSON: JSON.stringify(resources),
      NVBES_IDENTITY_RATE_LIMIT_KEY: randomBytes(32).toString('base64'),
    });
    // Even an accidental future Stripe request cannot reach an external service.
    environments.billing.NVBES_STRIPE_API_BASE_URL = 'http://127.0.0.1:1';
    for (const service of Object.keys(binaries)) {
      await command(binaries[service], ['migrate'], environments[service]);
    }
    const email = `runtime-${randomUUID()}@example.invalid`;
    const password = `Recovered-runtime-${secret()}!`;
    const seed = JSON.parse(
      await command(binaries.identity, ['synthetic-auth-smoke'], {
        ...environments.identity,
        NVBES_IDENTITY_SYNTHETIC_EMAIL: email,
        NVBES_IDENTITY_SYNTHETIC_PASSWORD: `Initial-runtime-${secret()}!`,
        NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD: password,
      }),
    );
    const identity = await fixture.start(
      binaries.identity,
      environments.identity,
      origins.identity,
      '/health/ready',
    );
    await fixture.start(binaries.account, environments.account, origins.account, '/health/ready');
    await fixture.start(binaries.billing, environments.billing, origins.billing, '/health/ready');

    // Explicit cookie jar models the hosted HTTP exchange; browser policy is tested separately.
    const cookies = new Map();
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
    let sessionCsrf;
    const issue = async (service, needsLogin) => {
      const verifier = secret();
      const state = secret();
      const query = new URLSearchParams({
        client_id: clients[0].client_id,
        redirect_uri: redirect,
        response_type: 'code',
        scope: `openid ${service}:read`,
        resource: origins[service],
        state,
        nonce: secret(),
        code_challenge: createHash('sha256').update(verifier).digest('base64url'),
        code_challenge_method: 'S256',
      });
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
        body: new URLSearchParams({
          grant_type: 'authorization_code',
          code: callback.searchParams.get('code'),
          client_id: clients[0].client_id,
          redirect_uri: redirect,
          code_verifier: verifier,
        }),
      });
      assert.equal(exchange.status, 200, `${service} token exchange`);
      const tokens = await exchange.json();
      assert.equal(tokens.token_type, 'Bearer');
      assert.equal(typeof tokens.access_token, 'string');
      return tokens.access_token;
    };
    const endpoints = {
      account: `${origins.account}/api/v1/profile`,
      billing: `${origins.billing}/workspaces/${seed.principal_id}/billing/overview`,
    };
    const access = async (service, token, expected) => {
      const response = await fetch(endpoints[service], {
        headers: { authorization: `Bearer ${token}` },
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(response.status, expected, `${service} protected request`);
      return response.json();
    };
    const account = await issue('account', true);
    const billing = await issue('billing', false);
    assert.equal((await access('account', account, 200)).user.id, seed.principal_id);
    assert.equal((await access('billing', billing, 200)).account_id, seed.principal_id);
    await access('account', billing, 401);
    await access('billing', account, 401);
    const logout = await post('/oauth/logout', {}, sessionCsrf);
    assert.equal(logout.status, 200);
    await logout.arrayBuffer();
    await access('account', account, 401);
    await access('billing', billing, 401);

    const nextAccount = await issue('account', true);
    const nextBilling = await issue('billing', false);
    await access('account', nextAccount, 200);
    await access('billing', nextBilling, 200);
    await identity.stop();
    await access('account', nextAccount, 503);
    await access('billing', nextBilling, 503);
    await fixture.start(
      binaries.identity,
      environments.identity,
      origins.identity,
      '/health/ready',
    );
    await access('account', nextAccount, 200);
    await access('billing', nextBilling, 200);
    await access('account', account, 401);
    await access('billing', billing, 401);
  },
);
