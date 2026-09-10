// Actual built Identity UI, real HTTP services and SDK; synthetic local credentials only.
export async function verifyHostedUi(browser, clientOrigin, screenshotPrefix) {
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    viewport: { width: 1280, height: 800 },
  });
  try {
    const page = await context.newPage();
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    const authorizationUrl = await page.evaluate(async (config) => {
      const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
      const { createAuthorizationRequest } =
        await import('/libs/ts/identity-sdk-web/src/oauth.authorization-request.ts');
      const request = await createAuthorizationRequest(
        {
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          resource: config.origins.account,
          storage: new BrowserSessionStorage(sessionStorage),
        },
        { scope: 'openid account:read' },
      );
      return request.authorizationUrl;
    }, config);
    let authorizationLoads = 0;
    let consentCalls = 0;
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.name));
    page.on('request', (request) => {
      if (
        new URL(request.url()).pathname === '/oauth/authorize' &&
        request.resourceType() === 'fetch'
      )
        authorizationLoads++;
      if (new URL(request.url()).pathname === '/oauth/authorize/approve') consentCalls++;
    });
    await page.goto(authorizationUrl);
    await page.getByLabel('Adresse email').waitFor();
    if (screenshotPrefix)
      await page.screenshot({ path: screenshotPrefix + '-desktop.png', fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth))
      throw new Error('Mobile layout overflows');
    if (screenshotPrefix)
      await page.screenshot({ path: screenshotPrefix + '-mobile.png', fullPage: true });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    if (await page.locator('input[type=password]').count())
      throw new Error('Password must be removed after login');
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    await page.waitForURL(config.redirectUri + '?**');
    const result = await page.evaluate(async (config) => {
      const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
      const { exchangeAuthorizationCode } =
        await import('/libs/ts/identity-sdk-web/src/oauth.authorization-code.ts');
      const { dpopFetch } = await import('/libs/ts/identity-sdk-web/src/dpop.ts');
      const query = new URL(location.href).searchParams;
      const tokens = await exchangeAuthorizationCode(
        {
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          storage: new BrowserSessionStorage(sessionStorage),
        },
        { code: query.get('code'), state: query.get('state') },
      );
      history.replaceState(null, '', '/callback');
      const response = await dpopFetch(config.origins.account + '/api/v1/profile', {
        key: tokens.dpopKey,
        accessToken: tokens.accessToken,
      });
      return {
        status: response.status,
        subjectMatches: tokens.identity.subject === config.subject,
      };
    }, config);
    if (
      authorizationLoads !== 1 ||
      consentCalls !== 1 ||
      result.status !== 200 ||
      !result.subjectMatches ||
      errors.length
    ) {
      throw new Error('Hosted UI authorization verification failed');
    }
    return {
      builtUi: true,
      authorizationLoads,
      consentCalls,
      accountStatus: result.status,
      subjectMatches: true,
      pageErrors: errors.length,
    };
  } finally {
    await context.close();
  }
}
