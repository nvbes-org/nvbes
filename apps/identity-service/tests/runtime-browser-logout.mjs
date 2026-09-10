import assert from 'node:assert/strict';

/** A second public client holds a real DPoP grant for the same Identity session. */
export async function verifyCrossClientLogout(page, config) {
  const sibling = await page.context().newPage();
  try {
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
    await page.getByRole('button', { name: 'Se déconnecter avec Identity' }).click();
    await page.getByRole('button', { name: 'Rester connecté' }).click();
    assert.equal(await billingStatus(), 200);
    await page.reload();
    await page.getByRole('button', { name: 'Confirmer la déconnexion' }).click();
    await page.getByRole('heading', { name: 'Vous êtes déconnecté.' }).waitFor();
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
    await page.reload();
    await page.getByText('Aucune session Identity à fermer', { exact: false }).waitFor();
    await page.goto(config.origins.client);
    await page.getByRole('button', { name: 'Se connecter avec Identity' }).click();
    await page.getByLabel('Adresse email').waitFor();
    return {
      identityLogoutConfirmed: true,
      cancelledLogoutPreservesAccess: true,
      siblingBillingStatusAfterLogout: revokedStatus,
      siblingRefreshRefused: true,
      freshLoginRequired: true,
    };
  } finally {
    await sibling.close();
  }
}
