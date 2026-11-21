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
          const { NvbesIdentityWeb } =
            await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
          const client = new NvbesIdentityWeb({
            baseUrl: config.origins.identity,
            clientId: config.clientId,
            redirectUri: config.redirectUri,
            resource: config.origins.account,
          });
          const transport = { baseUrl: config.origins.identity };
          const pending = parseHostedInteraction(interaction);
          const state =
            method === 'password'
              ? await loginHostedPassword(transport, pending, {
                  email: config.email,
                  password: config.password,
                })
              : await client.loginPasskey(pending);
          if (method === 'password') {
            const id = await client.registerPasskey(state.sessionCsrfToken, 'Chromium test key');
            if (!id) throw new Error('Credential registration failed');
          }
          const expiry = await client.stepUpPasskey(state.sessionCsrfToken);
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
        window.fixturePasskeyAccess = {
          tokens,
          dpopFetch,
          endpoint: `${config.origins.account}/api/v1/profile`,
        };
        return { status: response.status, subject: tokens.identity.subject };
      }, config);
      check(
        result.status === 200 && result.subject === config.subject,
        'Authenticated API identity must match',
      );
      if (method === 'passkey') {
        const management = await context.newPage();
        const secondCdp = await context.newCDPSession(management);
        await secondCdp.send('WebAuthn.enable');
        await secondCdp.send('WebAuthn.addVirtualAuthenticator', {
          options: {
            protocol: 'ctap2',
            transport: 'internal',
            hasResidentKey: true,
            hasUserVerification: true,
            isUserVerified: true,
            automaticPresenceSimulation: true,
          },
        });
        await management.goto(`${config.origins.identity}/__fixture/hosted`);
        await management.evaluate(
          async ({ csrf, config }) => {
            const { NvbesIdentityWeb } =
              await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
            const client = new NvbesIdentityWeb({
              baseUrl: config.origins.identity,
              clientId: config.clientId,
              redirectUri: config.redirectUri,
              resource: config.origins.account,
            });
            const { HostedIdentityError } =
              await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
            const keys = await client.listPasskeys(csrf);
            if (keys.length !== 1 || !keys[0].lastUsedAt)
              throw new Error('Used key metadata required');
            const first = keys[0].id;
            await client.renamePasskey(csrf, first, 'Renamed in Chromium');
            const renamed = await client.listPasskeys(csrf);
            if (renamed[0].label !== 'Renamed in Chromium') throw new Error('Rename not persisted');
            let lastFactorRefused = false;
            try {
              await client.revokePasskey(csrf, first);
            } catch (error) {
              lastFactorRefused = error instanceof HostedIdentityError && error.status === 409;
            }
            if (!lastFactorRefused) throw new Error('Last strong factor must be preserved');
            const second = await client.registerPasskey(csrf, 'Recovery key');
            const both = await client.listPasskeys(csrf);
            if (both.length !== 2 || !both.some((key) => key.id === second))
              throw new Error('Second key not persisted');
            await client.revokePasskey(csrf, first);
            let sessionRefused = false;
            try {
              await client.listPasskeys(csrf);
            } catch (error) {
              sessionRefused = error instanceof HostedIdentityError && error.status === 400;
            }
            if (!sessionRefused)
              throw new Error('Revoking the authenticating key must end its session');
          },
          { csrf: sessionCsrf, config },
        );
        const revoked = await page.evaluate(async () => {
          const { tokens, dpopFetch, endpoint } = window.fixturePasskeyAccess;
          const response = await dpopFetch(endpoint, {
            key: tokens.dpopKey,
            accessToken: tokens.accessToken,
          });
          return response.status === 401;
        });
        check(revoked, 'Credential revocation must invalidate the issued API access token');
        await management.close();
        continue;
      }
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
      credentialManagement: true,
      credentialRevocationEndsSessionAndApi: true,
    };
  } finally {
    await context.close();
  }
}
