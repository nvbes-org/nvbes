import assert from 'node:assert/strict';

/** A second public client holds a real DPoP grant for the same Identity session. */
export async function verifyCrossClientLogout(page, config, observeAccount = false) {
  const sibling = await page.context().newPage();
  const accountObserver = observeAccount ? await page.context().newPage() : null;
  try {
    // Playwright forces focus by default; disable it to exercise actual tab activation.
    for (const target of [page, sibling, accountObserver].filter(Boolean)) {
      const devtools = await page.context().newCDPSession(target);
      await devtools.send('Emulation.setFocusEmulationEnabled', { enabled: false });
    }
    await sibling.goto(config.origins.client2);
    const authorization = await sibling.evaluate(async (config) => {
      const { createAuthorizationRequest, BrowserSessionStorage } =
        await import('/libs/ts/identity-sdk-web/src/oauth.ts');
      const request = await createAuthorizationRequest(
        {
          baseUrl: config.origins.identity,
          clientId: config.billingClientId,
          redirectUri: `${config.origins.client2}/callback`,
          resource: config.origins.billing,
          storage: new BrowserSessionStorage(sessionStorage),
        },
        { scope: 'openid billing:read offline_access' },
      );
      return request.authorizationUrl;
    }, config);
    await sibling.goto(authorization);
    await sibling.getByRole('button', { name: 'Autoriser et continuer' }).click();
    await sibling.waitForURL(`${config.origins.client2}/callback?**`);
    await sibling.evaluate(async (config) => {
      const { exchangeAuthorizationCode, BrowserSessionStorage, OAuthSession } =
        await import('/libs/ts/identity-sdk-web/src/oauth.ts');
      const url = new URL(location.href);
      const tokens = await exchangeAuthorizationCode(
        {
          baseUrl: config.origins.identity,
          clientId: config.billingClientId,
          redirectUri: `${config.origins.client2}/callback`,
          storage: new BrowserSessionStorage(sessionStorage),
        },
        { code: url.searchParams.get('code'), state: url.searchParams.get('state') },
      );
      history.replaceState(null, '', '/callback');
      window.fixtureBillingSession = new OAuthSession(
        { baseUrl: config.origins.identity, clientId: config.billingClientId },
        tokens,
      );
    }, config);
    const billingStatus = () =>
      sibling.evaluate(async (config) => {
        const { dpopFetch } = await import('/libs/ts/identity-sdk-web/src/dpop.ts');
        const tokens = window.fixtureBillingSession.snapshot();
        const response = await dpopFetch(
          `${config.origins.billing}/accounts/principal/${config.subject}/billing/overview`,
          {
            accessToken: tokens.accessToken,
            key: tokens.dpopKey,
            cache: 'no-store',
          },
        );
        return response.status;
      }, config);
    assert.equal(await billingStatus(), 200);
    if (accountObserver) {
      await accountObserver.goto(config.origins.client);
      await accountObserver.getByRole('button', { name: 'Se connecter avec Identity' }).click();
      await accountObserver.getByRole('button', { name: 'Autoriser et continuer' }).click();
      await accountObserver.getByRole('heading', { name: 'Votre profil.' }).waitFor();
      // Cross-origin OAuth navigations create a new renderer and reapply Playwright emulation.
      const devtools = await page.context().newCDPSession(accountObserver);
      await devtools.send('Emulation.setFocusEmulationEnabled', { enabled: false });
    }
    await page.bringToFront();
    if (accountObserver) await accountObserver.waitForFunction(() => !document.hasFocus());
    await page.getByRole('button', { name: 'Se déconnecter avec Identity' }).click();
    await page.getByRole('button', { name: 'Rester connecté' }).click();
    assert.equal(await billingStatus(), 200);
    // Cancellation discards the in-memory request; start a fresh RP request.
    await page.goto(config.origins.client);
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).click();
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    await page.getByRole('heading', { name: 'Votre profil.' }).waitFor();
    const submission = page.waitForRequest(
      (request) => new URL(request.url()).pathname === '/oauth/end-session',
    );
    await page.getByRole('button', { name: 'Se déconnecter avec Identity' }).click();
    const sent = await submission;
    assert.equal(sent.method(), 'POST');
    assert.equal(new URL(sent.url()).search, '');
    assert.ok(new URLSearchParams(sent.postData()).has('id_token_hint'));
    await page.getByRole('button', { name: 'Confirmer la déconnexion' }).click();
    await page.getByText('Vous êtes déconnecté de votre session Identity.').waitFor();
    assert.equal(page.url(), `${config.origins.client}/`);
    assert.deepEqual(await page.evaluate(() => Object.keys(sessionStorage)), []);
    const revokedStatus = await billingStatus();
    assert.equal(revokedStatus, 401);
    const refreshRefused = await sibling.evaluate(async () => {
      try {
        await window.fixtureBillingSession.refresh();
        return false;
      } catch {
        return window.fixtureBillingSession.snapshot() === null;
      }
    });
    assert.equal(refreshRefused, true);
    if (accountObserver) {
      const revalidation = accountObserver.waitForResponse(
        (response) => response.url() === `${config.origins.account}/api/v1/profile`,
      );
      await accountObserver.bringToFront();
      assert.equal((await revalidation).status(), 401);
      await accountObserver
        .getByText('La connexion ou la lecture du compte n’a pas abouti.', { exact: false })
        .waitFor();
      assert.equal(
        await accountObserver.getByRole('heading', { name: 'Votre profil.' }).count(),
        0,
      );
    }
    await page.goto(`${config.origins.client}/oauth/logout/callback?state=forged`);
    await page
      .getByText('La connexion ou la lecture du compte n’a pas abouti.', { exact: false })
      .waitFor();
    assert.equal(page.url(), `${config.origins.client}/`);
    await page.goto(`${config.origins.identity}/logout`);
    await page.getByText('Aucune session Identity à fermer', { exact: false }).waitFor();
    await page.goto(config.origins.client);
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).click();
    await page.getByLabel('Adresse email').waitFor();
    return {
      identityLogoutConfirmed: true,
      rpLogoutPostAndValidatedReturn: true,
      forgedLogoutCallbackRefused: true,
      cancelledLogoutPreservesAccess: true,
      siblingBillingStatusAfterLogout: revokedStatus,
      siblingRefreshRefused: true,
      freshLoginRequired: true,
      openAccountProfileClearedOnReturn: accountObserver ? true : 'not-run',
    };
  } finally {
    await sibling.close();
    await accountObserver?.close();
  }
}
