import { readFile, readdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';

export async function accountWebBuild(config) {
  const root = resolve('apps/account-web/dist');
  const index = await readFile(resolve(root, 'index.html'));
  const assets = new Map();
  for (const name of await readdir(resolve(root, 'assets'))) {
    const type = { js: 'text/javascript', css: 'text/css', woff2: 'font/woff2' }[
      name.split('.').at(-1)
    ];
    if (type)
      assets.set(`/assets/${name}`, { type, body: await readFile(resolve(root, 'assets', name)) });
  }
  return (request, response) => {
    if (request.method !== 'GET') return false;
    const url = new URL(request.url, config.origins.client);
    const document =
      ['/', '/oauth/callback'].includes(url.pathname) &&
      request.headers.accept?.includes('text/html');
    const asset = assets.get(url.pathname);
    const configuration = url.pathname === '/account-config.json';
    if (!document && !asset && !configuration) return false;
    response.writeHead(200, {
      'content-type': configuration
        ? 'application/json'
        : document
          ? 'text/html; charset=utf-8'
          : asset.type,
      'cache-control': 'no-store',
      'referrer-policy': 'no-referrer',
      'x-content-type-options': 'nosniff',
      'content-security-policy': `default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; img-src 'self'; connect-src 'self' ${config.origins.identity} ${config.origins.account}; base-uri 'none'; frame-ancestors 'none'; form-action 'self'`,
    });
    response.end(
      configuration
        ? JSON.stringify({
            identityOrigin: config.origins.identity,
            accountApiOrigin: config.origins.account,
            clientId: config.clientId,
          })
        : document
          ? index
          : asset.body,
    );
    return true;
  };
}

/** Actual built sites, real services and isolated synthetic credentials. */
export async function verifyAccountWeb(config) {
  const { chromium } = createRequire(resolve('apps/identity-web/package.json'))('playwright');
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', () => errors.push('browser page error'));
    let exchanges = 0;
    page.on('request', (request) => {
      if (new URL(request.url()).pathname === '/oauth/token') exchanges++;
    });
    await page.goto(config.origins.client);
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).click();
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    const profileResponse = page.waitForResponse(
      (response) => response.url() === `${config.origins.account}/api/v1/profile`,
    );
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    await page.getByRole('heading', { name: 'Votre profil.' }).waitFor();
    const response = await profileResponse;
    assert.equal(response.status(), 200);
    assert.equal((await response.json()).user.id, config.subject);
    assert.equal(page.url(), `${config.origins.client}/`);
    assert.equal(exchanges, 1);
    assert.deepEqual(
      await page.evaluate(() => ({
        session: Object.keys(sessionStorage),
        local: Object.keys(localStorage),
      })),
      { session: [], local: [] },
    );
    await page.getByRole('button', { name: 'Fermer Account sur cette page' }).click();
    await page.getByText('Account est fermé sur cette page.', { exact: false }).waitFor();
    assert.equal(await page.getByRole('heading', { name: 'Votre profil.' }).count(), 0);
    await page.reload();
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).waitFor();
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    await page.goto(`${config.origins.client}/oauth/callback?code=hostile&state=wrong`);
    await page
      .getByText('La connexion ou la lecture du compte n’a pas abouti.', { exact: false })
      .waitFor();
    assert.equal(exchanges, 1);
    assert.equal(page.url(), `${config.origins.client}/`);
    assert.deepEqual(errors, []);
    return {
      browser: browser.version(),
      realAccountWeb: true,
      accountStatus: 200,
      subjectMatches: true,
      callbackExchangedOnce: true,
      persistentTokensAbsent: true,
      invalidCallbackRefused: true,
      localCloseClearsProfile: true,
      mobileOverflow: false,
    };
  } finally {
    await browser.close();
  }
}
