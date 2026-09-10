import { generateKeyPairSync, randomBytes, randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import { createServer } from 'vite-plus';
import { command, RuntimeFixture, runtimeEnvironment, unusedPort } from './runtime-processes.mjs';
import { testCertificates, tlsEndpoint } from './runtime-browser-tls.mjs';
import { identityWebBuild } from './runtime-browser-web-ui.mjs';
import { verifyHostedReauthentication } from './runtime-browser-hosted-reauth.mjs';
import { accountWebBuild, verifyAccountWeb } from './runtime-browser-account-web.mjs';

const fixture = new RuntimeFixture();
let closed = false;
async function close() {
  if (closed) return;
  closed = true;
  clearTimeout(deadline);
  clearInterval(parentMonitor);
  await fixture.close();
}
const deadline = setTimeout(() => {
  void close();
}, 10 * 60_000);
const launcher = process.ppid;
const parentMonitor = setInterval(() => {
  if (process.ppid !== launcher) void close();
}, 1000);
process.once('SIGTERM', () => {
  void close();
});
process.once('SIGINT', () => {
  void close();
});

try {
  const web = process.env.NVBES_IDENTITY_TEST_WEB_UI === '1' ? await identityWebBuild() : undefined;
  const accountWeb = process.env.NVBES_IDENTITY_TEST_ACCOUNT_WEB === '1';
  if (accountWeb && !web) throw new Error('Account verification requires the built Identity UI');
  if (process.env.NVBES_IDENTITY_TEST_WEB_REAUTH === '1' && !web)
    throw new Error('Reauthentication verification requires the built Identity UI');
  const tls = await testCertificates(fixture);
  const databases = await fixture.database();
  const vite = await createServer({
    configFile: false,
    root: process.cwd(),
    resolve: { alias: { '@nvbes/http-client': resolve('libs/ts/http-client/src/index.ts') } },
    appType: 'custom',
    server: { middlewareMode: true, hmr: false },
    logLevel: 'error',
  });
  fixture.cleanups.push(() => vite.close());
  const origins = {};
  const backends = {};
  const binaries = {
    identity: resolve('target/debug/nvbes-identity-service'),
    account: resolve('target/debug/nvbes-account-service'),
    billing: resolve('apps/billing-service/target/debug/nvbes-billing-service'),
  };
  for (const service of ['identity', 'account', 'billing', 'client', 'client2', 'unregistered']) {
    origins[service] =
      `https://${service === 'identity' ? 'localhost' : '127.0.0.1'}:${await unusedPort()}`;
    if (binaries[service]) backends[service] = `http://127.0.0.1:${await unusedPort()}`;
  }
  const secret = () => randomBytes(32).toString('base64url');
  const { privateKey, publicKey } = generateKeyPairSync('rsa', {
    modulusLength: 2048,
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    publicKeyEncoding: { type: 'spki', format: 'pem' },
  });
  const resources = ['account', 'billing'].map((service) => ({
    client_id: `${service}-browser-test`,
    audience: `nvbes-${service}-service`,
    secret: secret(),
  }));
  const clientId = 'https-browser-test';
  const redirectUri = `${origins.client}${accountWeb ? '/oauth/callback' : '/callback'}`;
  const clients = [
    {
      client_id: clientId,
      display_name: 'HTTPS browser test',
      redirect_uris: [redirectUri],
      post_logout_redirect_uris: accountWeb ? [`${origins.client}/oauth/logout/callback`] : [],
      allow_refresh: true,
      require_dpop: true,
      resources: Object.fromEntries(
        ['account', 'billing'].map((service) => [
          origins[service],
          {
            audience: `nvbes-${service}-service`,
            scopes: [`${service}:read`],
          },
        ]),
      ),
    },
  ];
  const authorizationSecret = randomBytes(32).toString('hex');
  const billingClientId = 'https-browser-billing-test';
  const mfaClientId = 'https-browser-mfa-test';
  const webauthnClientId = 'https-browser-webauthn-test';
  for (const [id, policy] of [
    [mfaClientId, 'recent_mfa'],
    [webauthnClientId, 'recent_webauthn'],
  ]) {
    clients.push({ ...clients[0], client_id: id, minimum_authentication: policy });
  }
  clients.push({
    ...clients[0],
    client_id: billingClientId,
    redirect_uris: [`${origins.client2}/callback`],
  });
  const environments = {};
  for (const service of Object.keys(binaries)) {
    const prefix = `NVBES_${service.toUpperCase()}`;
    environments[service] = {
      ...runtimeEnvironment(),
      SSL_CERT_FILE: tls.caPath,
      [`${prefix}_DATABASE_URL`]: databases[service],
      [`${prefix}_BIND_ADDR`]: `127.0.0.1:${new URL(backends[service]).port}`,
      [`${prefix}_PUBLIC_ORIGIN`]: origins[service],
      [`${prefix}_BROWSER_ORIGINS_JSON`]: JSON.stringify([
        service === 'billing' ? origins.client2 : origins.client,
      ]),
      NVBES_IDENTITY_TOKEN_ISSUER: origins.identity,
      NVBES_IDENTITY_TOKEN_KEY_ID: 'https-browser-key',
      NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: publicKey,
      NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET: authorizationSecret,
    };
    const resource = resources.find((value) => value.audience === `nvbes-${service}-service`);
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
  Object.assign(environments.billing, {
    NVBES_BILLING_ACCOUNT_ORIGIN: origins.account,
    NVBES_STRIPE_API_BASE_URL: 'http://127.0.0.1:1',
  });
  for (const service of Object.keys(binaries))
    await command(binaries[service], ['migrate'], environments[service]);
  const email = `https-${randomUUID()}@example.invalid`;
  const password = `Recovered-browser-${secret()}!`;
  const seed = JSON.parse(
    await command(binaries.identity, ['synthetic-auth-smoke'], {
      ...environments.identity,
      NVBES_IDENTITY_SYNTHETIC_EMAIL: email,
      NVBES_IDENTITY_SYNTHETIC_PASSWORD: `Initial-${secret()}!`,
      NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD: password,
    }),
  );
  const config = {
    origins,
    clientId,
    billingClientId,
    mfaClientId,
    webauthnClientId,
    redirectUri,
    email,
    password,
    subject: seed.principal_id,
  };
  const accountSite = accountWeb ? await accountWebBuild(config) : undefined;
  for (const service of Object.keys(origins))
    await tlsEndpoint({
      fixture,
      tls,
      origin: origins[service],
      backend: backends[service],
      middleware: vite.middlewares,
      web: service === 'identity' ? web : service === 'client' ? accountSite : undefined,
      config,
    });
  for (const service of Object.keys(binaries))
    await fixture.start(
      binaries[service],
      environments[service],
      backends[service],
      '/health/ready',
    );
  // Only public test URLs are printed. Keys and synthetic credentials stay inside the fixture.
  console.log(JSON.stringify({ client: origins.client, pid: process.pid, expiresInSeconds: 600 }));
  if (accountWeb) {
    console.log(JSON.stringify(await verifyAccountWeb(config)));
    await close();
  }
  if (process.env.NVBES_IDENTITY_TEST_WEB_REAUTH === '1') {
    console.log(JSON.stringify(await verifyHostedReauthentication(fixture, origins.client)));
    await close();
  }
} catch (error) {
  await close();
  throw error;
}
