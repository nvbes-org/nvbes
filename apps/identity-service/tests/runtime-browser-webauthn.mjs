// Real Identity/Account runtimes and browser SDK; only the CTAP2 device is virtual.
export async function verifyBrowserWebauthn(browser, clientOrigin) {
  const check = (condition, message) => {
    if (!condition) throw new Error(message);
  };
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('WebAuthn.enable');
    const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'internal',
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    let sessionCsrf;
    for (const method of ['password', 'passkey']) {
      await page.goto(clientOrigin);
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
      const response = await page.goto(authorizationUrl);
      check(response.status() === 200, 'Authorization must succeed');
      const interaction = await response.json();
      check(interaction.needs_login === true, 'Each method must create a fresh session');
      await page.goto(`${config.origins.identity}/__fixture/hosted`);
      const authenticated = await page.evaluate(
        async ({ config, interaction, method }) => {
          const { parseHostedInteraction, loginHostedPassword } =
            await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
          const { registerHostedPasskey, loginHostedPasskey, stepUpHostedPasskey } =
            await import('/libs/ts/identity-sdk-web/src/hosted.webauthn.ts');
          const transport = { baseUrl: config.origins.identity };
          const pending = parseHostedInteraction(interaction);
          const state =
            method === 'password'
              ? await loginHostedPassword(transport, pending, {
                  email: config.email,
                  password: config.password,
                })
              : await loginHostedPasskey(transport, pending);
          if (method === 'password') {
            const id = await registerHostedPasskey(
              transport,
              state.sessionCsrfToken,
              'Chromium test key',
            );
            if (!id) throw new Error('Credential registration failed');
          }
          const expiry = await stepUpHostedPasskey(transport, state.sessionCsrfToken);
          if (Date.parse(expiry) <= Date.now()) throw new Error('Step-up must be fresh');
          return state;
        },
        { config, interaction, method },
      );
      sessionCsrf = authenticated.sessionCsrfToken;
      const redirect = await page.evaluate(async (state) => {
        const { completeHostedConsent } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        return completeHostedConsent({ baseUrl: location.origin }, state, 'approve');
      }, authenticated);
      await page.goto(redirect);
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
        const response = await dpopFetch(`${config.origins.account}/api/v1/profile`, {
          key: tokens.dpopKey,
          accessToken: tokens.accessToken,
        });
        return { status: response.status, subject: tokens.identity.subject };
      }, config);
      check(
        result.status === 200 && result.subject === config.subject,
        'Authenticated API identity must match',
      );
      await page.goto(`${config.origins.identity}/__fixture/hosted`);
      await page.evaluate(async (csrf) => {
        const { logoutHostedSession } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        await logoutHostedSession({ baseUrl: location.origin }, csrf);
      }, sessionCsrf);
    }
    const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
    check(
      credentials.length === 1 && credentials[0].isResidentCredential,
      'Discoverable credential required',
    );
    check(credentials[0].signCount >= 3, 'Registration must be followed by real signed assertions');
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    return {
      browser: browser.version(),
      registration: true,
      passwordlessLogin: true,
      stepUp: true,
      oidcAndAccount: true,
      virtualAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
