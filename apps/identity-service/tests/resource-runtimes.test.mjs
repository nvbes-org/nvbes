import assert from 'node:assert/strict';
import { generateKeyPairSync, randomBytes, randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import test from 'node:test';
import { command, RuntimeFixture, runtimeEnvironment, unusedPort } from './runtime-processes.mjs';
import { verifyBillingAuthorization } from './runtime-billing-authorization.mjs';
import { verifyResourceDpop } from './runtime-resource-dpop.mjs';
import { oauthClient } from './runtime-oauth-client.mjs';
import { verifySdkRefresh } from './runtime-sdk-refresh.mjs';
import { verifyIdentityCors } from './runtime-identity-cors.mjs';
import { verifyResourceCors } from './runtime-resource-cors.mjs';

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
              scopes:
                service === 'account'
                  ? ['account:read', 'account:write']
                  : ['billing:read', 'billing:checkout'],
            },
          ]),
        ),
        allow_refresh: true,
        require_dpop: false,
      },
    ];
    for (const service of Object.keys(binaries)) {
      const prefix = `NVBES_${service.toUpperCase()}`;
      environments[service] = {
        ...runtimeEnvironment(),
        [`${prefix}_DATABASE_URL`]: databases[service],
        [`${prefix}_BIND_ADDR`]: `127.0.0.1:${new URL(origins[service]).port}`,
        [`${prefix}_PUBLIC_ORIGIN`]: origins[service],
        [`${prefix}_BROWSER_ORIGINS_JSON`]: JSON.stringify([origins.account]),
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
    const billingAuthorizationSecret = randomBytes(32).toString('hex');
    environments.account.NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET = billingAuthorizationSecret;
    environments.billing.NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET = billingAuthorizationSecret;
    environments.billing.NVBES_BILLING_ACCOUNT_ORIGIN = origins.account;
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
    let accountRuntime = await fixture.start(
      binaries.account,
      environments.account,
      origins.account,
      '/health/ready',
    );
    let billingRuntime = await fixture.start(
      binaries.billing,
      environments.billing,
      origins.billing,
      '/health/ready',
    );

    await verifyIdentityCors(origins);
    const browser = oauthClient({
      origins,
      clientId: clients[0].client_id,
      redirect,
      email,
      password,
    });
    const { issue } = browser;
    const endpoints = {
      account: `${origins.account}/api/v1/profile`,
      billing: `${origins.billing}/accounts/principal/${seed.principal_id}/billing/overview`,
    };
    await verifyResourceCors(origins, endpoints);
    const access = async (service, token, expected) => {
      const response = await fetch(endpoints[service], {
        headers: { authorization: `Bearer ${token}`, origin: origins.account },
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(response.status, expected, `${service} protected request`);
      assert.equal(response.headers.get('access-control-allow-origin'), origins.account);
      return response.json();
    };
    const account = await issue('account', true);
    const billing = await issue('billing', false);
    assert.equal((await access('account', account, 200)).user.id, seed.principal_id);
    assert.equal((await access('billing', billing, 200)).account_id, seed.principal_id);
    await access('account', billing, 401);
    await access('billing', account, 401);
    const accountWrite = await issue('account', false, 'account:read account:write');
    const billingCheckout = await issue('billing', false, 'billing:read billing:checkout');
    await verifyBillingAuthorization({
      fixture,
      origins,
      principalId: seed.principal_id,
      accountToken: accountWrite,
      billingToken: billingCheckout,
      readOnlyToken: billing,
    });
    await accountRuntime.stop();
    await access('billing', billing, 503);
    accountRuntime = await fixture.start(
      binaries.account,
      environments.account,
      origins.account,
      '/health/ready',
    );
    await access('billing', billing, 200);
    const boundTokens = await verifyResourceDpop({
      fixture,
      endpoints,
      issue,
      restart: async (service) => {
        await (service === 'account' ? accountRuntime : billingRuntime).stop();
        const runtime = await fixture.start(
          binaries[service],
          environments[service],
          origins[service],
          '/health/ready',
        );
        if (service === 'account') accountRuntime = runtime;
        else billingRuntime = runtime;
      },
    });
    const sdkSessions = await verifySdkRefresh({
      origins,
      endpoints,
      browser,
      clientId: clients[0].client_id,
      redirect,
    });
    const logout = await browser.logout();
    assert.equal(logout.status, 200);
    await logout.arrayBuffer();
    for (const session of sdkSessions) {
      await assert.rejects(session.refresh(), /refresh failed/);
      assert.equal(session.snapshot(), null);
    }
    await access('account', account, 401);
    await access('billing', billing, 401);
    for (const { service, token, key } of boundTokens) {
      const response = await fetch(endpoints[service], {
        headers: {
          authorization: `DPoP ${token}`,
          dpop: key.proof('GET', endpoints[service], token),
        },
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(response.status, 401, 'logout revokes bound resource tokens');
      await response.arrayBuffer();
    }

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
