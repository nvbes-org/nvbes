// Real services and SDK; the non-resident CTAP2 authenticator is virtual.
export async function verifyBrowserSecurityKey(browser, clientOrigin) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('WebAuthn.enable');
    const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'usb',
        hasResidentKey: false,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    for (const phase of ['enroll', 'cancel', 'login']) {
      await page.goto(clientOrigin);
      const authorizationUrl = await page.evaluate(async (config) => {
        const { createAuthorizationRequest } =
          await import('/libs/ts/identity-sdk-web/src/oauth.authorization-request.ts');
        const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
        const storage = new BrowserSessionStorage(sessionStorage);
        // Previous setup/cancel interactions were deliberately not approved.
        storage.clearTransaction();
        return (
          await createAuthorizationRequest(
            {
              baseUrl: config.origins.identity,
              clientId: config.clientId,
              redirectUri: config.redirectUri,
              resource: config.origins.account,
              storage,
            },
            { scope: 'openid account:read' },
          )
        ).authorizationUrl;
      }, config);
      const response = await page.goto(authorizationUrl);
      if (response.status() !== 200) throw new Error('Authorization failed');
      const interaction = await response.json();
      if (interaction.needs_login !== true) throw new Error('Previous session was not revoked');
      await page.goto(`${config.origins.identity}/__fixture/hosted`);
      const result = await page.evaluate(
        async ({ config, interaction, phase }) => {
          const { parseHostedInteraction, loginHostedPassword, completeHostedConsent } =
            await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
          const { NvbesIdentityWeb } =
            await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
          const { HostedSecurityKeyLoginError } =
            await import('/libs/ts/identity-sdk-web/src/hosted.security-key.ts');
          const client = new NvbesIdentityWeb({
            baseUrl: config.origins.identity,
            clientId: config.clientId,
            redirectUri: config.redirectUri,
            resource: config.origins.account,
          });
          const transport = { baseUrl: config.origins.identity };
          const state = parseHostedInteraction(interaction);
          const credentials = { email: config.email, password: config.password };
          if (phase === 'enroll') {
            const authenticated = await loginHostedPassword(transport, state, credentials);
            await client.registerPasskey(authenticated.sessionCsrfToken, 'Non-resident USB key');
            await client.logout(authenticated.sessionCsrfToken);
            return null;
          }
          if (phase === 'cancel') {
            const controller = new AbortController();
            controller.abort();
            try {
              await client.loginWithSecurityKey(state, credentials, { signal: controller.signal });
            } catch (error) {
              if (error instanceof HostedSecurityKeyLoginError && error.sessionRevocationConfirmed)
                return null;
              throw error;
            }
            throw new Error('Cancelled key login unexpectedly succeeded');
          }
          const authenticated = await client.loginWithSecurityKey(state, credentials);
          if (Date.parse(authenticated.stepUpExpiresAt) <= Date.now())
            throw new Error('Strong authentication is not fresh');
          return completeHostedConsent(transport, authenticated.interaction, 'approve');
        },
        { config, interaction, phase },
      );
      if (phase !== 'login') continue;
      await page.goto(result);
      const access = await page.evaluate(async (config) => {
        const { exchangeAuthorizationCode } =
          await import('/libs/ts/identity-sdk-web/src/oauth.authorization-code.ts');
        const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
        const { dpopFetch } = await import('/libs/ts/identity-sdk-web/src/dpop.ts');
        const params = new URL(location.href).searchParams;
        const tokens = await exchangeAuthorizationCode(
          {
            baseUrl: config.origins.identity,
            clientId: config.clientId,
            redirectUri: config.redirectUri,
            storage: new BrowserSessionStorage(sessionStorage),
          },
          { code: params.get('code'), state: params.get('state') },
        );
        history.replaceState(null, '', '/callback');
        const response = await dpopFetch(`${config.origins.account}/api/v1/profile`, {
          key: tokens.dpopKey,
          accessToken: tokens.accessToken,
        });
        return { status: response.status, subject: tokens.identity.subject };
      }, config);
      if (access.status !== 200 || access.subject !== config.subject)
        throw new Error('Account access mismatch');
    }
    const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
    if (
      credentials.length !== 1 ||
      credentials[0].isResidentCredential ||
      credentials[0].signCount < 1
    )
      throw new Error('Expected signed assertions from a non-resident key');
    return {
      browser: browser.version(),
      nonResidentCredential: true,
      passwordThenSecurityKey: true,
      cancellationRevokesPasswordSession: true,
      oidcAndAccount: true,
      virtualAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
