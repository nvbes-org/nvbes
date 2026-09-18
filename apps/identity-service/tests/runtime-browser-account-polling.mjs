import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';

/** Real timers and HTTP: logout elsewhere while Account receives no focus event. */
export async function verifyAccountPolling(config) {
  const { chromium } = createRequire(resolve('apps/account-web/package.json'))('playwright');
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    const actor = await context.newPage();
    await actor.goto(`${config.origins.identity}/__fixture/hosted`);
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', () => errors.push('page error'));
    await page.goto(config.origins.client);
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).click();
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Se connecter', exact: true }).click();
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    await page.getByRole('heading', { name: 'Votre profil.' }).waitFor();
    const visible = await page.evaluate(() => {
      window.fixtureActivityEvents = 0;
      window.addEventListener('focus', () => window.fixtureActivityEvents++);
      document.addEventListener('visibilitychange', () => window.fixtureActivityEvents++);
      return document.visibilityState === 'visible';
    });
    assert.equal(visible, true);
    const revokedResponse = page.waitForResponse(
      (response) =>
        response.url() === `${config.origins.account}/api/v1/profile` && response.status() === 401,
      { timeout: 75_000 },
    );
    const started = Date.now();
    // Script execution on the existing Identity page does not activate its tab.
    // Cookies, Origin, CSRF and the real logout endpoint remain authoritative.
    await actor.evaluate(async () => {
      const { loadHostedLogoutContext } =
        await import('/libs/ts/identity-sdk-web/src/hosted.logout.ts');
      const { logoutHostedSession } =
        await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
      const transport = { baseUrl: location.origin };
      const csrf = await loadHostedLogoutContext(transport);
      if (!csrf) throw new Error('Missing shared Identity session');
      await logoutHostedSession(transport, csrf);
    });
    await revokedResponse;
    await page
      .getByText('La connexion ou la lecture du compte n’a pas abouti.', { exact: false })
      .waitFor();
    const elapsed = Date.now() - started;
    const state = await page.evaluate(() => ({
      activityEvents: window.fixtureActivityEvents,
      visible: document.visibilityState === 'visible',
      persistentEntries: localStorage.length + sessionStorage.length,
    }));
    assert.equal(await page.getByRole('heading', { name: 'Votre profil.' }).count(), 0);
    assert.deepEqual(state, { activityEvents: 0, visible: true, persistentEntries: 0 });
    assert.deepEqual(errors, []);
    assert.ok(elapsed >= 50_000 && elapsed <= 75_000);
    return {
      browser: browser.version(),
      realTimers: true,
      continuouslyVisible: true,
      focusOrVisibilityEvents: 0,
      logoutElsewhereConfirmed: true,
      periodicAccountStatus: 401,
      profileCleared: true,
      elapsedMs: elapsed,
    };
  } finally {
    await browser.close();
  }
}
