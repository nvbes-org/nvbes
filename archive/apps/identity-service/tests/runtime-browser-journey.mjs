// Called with a Playwright Browser and the URL printed by runtime-browser-fixture.mjs.
// This fixture is not a product UI. Every protocol request reaches the real services.
export async function verifyBrowserJourney(browser, clientOrigin) {
  const check = (condition, message) => {
    if (!condition) throw new Error(message);
  };
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const sessions = [];
  const keyThumbprints = new Set();
  let sessionCsrf;
  try {
    const firstPage = await context.newPage();
    await firstPage.goto(clientOrigin);
    const config = await firstPage.evaluate(async () => (await fetch('/__fixture/config')).json());
    const fixtureConfig = config;
    check(
      await firstPage.evaluate(
        (identity) => new URL(identity).hostname !== location.hostname,
        config.origins.identity,
      ),
      'Identity and client must be different sites',
    );
    for (const service of ['account', 'billing']) {
      const config =
        service === 'account'
          ? fixtureConfig
          : {
              ...fixtureConfig,
              clientId: fixtureConfig.billingClientId,
              redirectUri: `${fixtureConfig.origins.client2}/callback`,
            };
      const clientOrigin = service === 'account' ? config.origins.client : config.origins.client2;
      const page = service === 'account' ? firstPage : await context.newPage();
      await page.goto(clientOrigin);
      const authorizationUrl = await page.evaluate(
        async ({ config, service }) => {
          const root = '/libs/ts/identity-sdk-web/src/';
          const { BrowserSessionStorage } = await import(`${root}storage.ts`);
          const { createAuthorizationRequest } = await import(
            `${root}oauth.authorization-request.ts`
          );
          const storage = new BrowserSessionStorage(sessionStorage);
          const request = await createAuthorizationRequest(
            {
              baseUrl: config.origins.identity,
              clientId: config.clientId,
              redirectUri: config.redirectUri,
              resource: config.origins[service],
              storage,
            },
            { scope: `openid offline_access ${service}:read` },
          );
          sessionStorage.setItem(
            'fixture.expected-key',
            JSON.stringify(storage.getTransaction().dpop),
          );
          return request.authorizationUrl;
        },
        { config, service },
      );
      // Actual cross-site top-level navigation, including the server's browser cookie.
      const response = await page.goto(authorizationUrl);
      check(response.status() === 200, 'Authorization must succeed');
      const interaction = await response.json();
      check(
        interaction.needs_login === (service === 'account'),
        'Identity session must survive the return to the client',
      );
      await page.goto(`${config.origins.identity}/__fixture/hosted`);
      const navigation = await page.evaluate(
        async ({ interaction, config }) => {
          const { parseHostedInteraction, loginHostedPassword, completeHostedConsent } =
            await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
          const transport = { baseUrl: config.origins.identity };
          let state = parseHostedInteraction(interaction);
          if (state.needsLogin)
            state = await loginHostedPassword(transport, state, {
              email: config.email,
              password: config.password,
            });
          return {
            redirectUri: await completeHostedConsent(transport, state, 'approve'),
            sessionCsrf: state.sessionCsrfToken,
          };
        },
        { interaction, config },
      );
      sessionCsrf = navigation.sessionCsrf ?? sessionCsrf;
      check(
        await page.evaluate(
          ({ destination, clientOrigin }) => new URL(destination).origin === clientOrigin,
          { destination: navigation.redirectUri, clientOrigin },
        ),
        'Consent destination must be registered',
      );
      await page.goto(navigation.redirectUri);
      const result = await page.evaluate(
        async ({ config, service }) => {
          const root = '/libs/ts/identity-sdk-web/src/';
          const { BrowserSessionStorage } = await import(`${root}storage.ts`);
          const { exchangeAuthorizationCode } = await import(`${root}oauth.authorization-code.ts`);
          const { OAuthSession } = await import(`${root}oauth.session.ts`);
          const { IndexedDbDpopTransactionStore } = await import(
            `${root}dpop.transaction-store.ts`
          );
          const { dpopFetch } = await import(`${root}dpop.ts`);
          const expected = JSON.parse(sessionStorage.getItem('fixture.expected-key'));
          const storage = new BrowserSessionStorage(sessionStorage);
          const query = new URL(location.href).searchParams;
          const tokens = await exchangeAuthorizationCode(
            {
              baseUrl: config.origins.identity,
              clientId: config.clientId,
              redirectUri: config.redirectUri,
              storage,
            },
            { code: query.get('code'), state: query.get('state') },
          );
          history.replaceState(null, '', '/callback');
          const session = new OAuthSession(
            { baseUrl: config.origins.identity, clientId: config.clientId },
            tokens,
          );
          const refreshed = await session.refresh();
          const endpoint =
            service === 'account'
              ? `${config.origins.account}/api/v1/profile`
              : `${config.origins.billing}/accounts/principal/${config.subject}/billing/overview`;
          const response = await dpopFetch(endpoint, {
            key: refreshed.dpopKey,
            accessToken: refreshed.accessToken,
          });
          const info = await dpopFetch(`${config.origins.identity}/oauth/userinfo`, {
            key: refreshed.dpopKey,
            accessToken: refreshed.accessToken,
          });
          const downgrade = await fetch(endpoint, {
            credentials: 'omit',
            headers: { authorization: `Bearer ${refreshed.accessToken}` },
          });
          let exportRejected = false;
          try {
            await crypto.subtle.exportKey('jwk', refreshed.dpopKey.keyPair.privateKey);
          } catch {
            exportRejected = true;
          }
          window.fixtureSession = { session, endpoint, dpopFetch };
          return {
            secure: isSecureContext,
            status: response.status,
            userinfo: info.status,
            downgrade: downgrade.status,
            subject: tokens.identity.subject,
            sameKey: expected.jkt === refreshed.dpopKey.jkt,
            thumbprint: refreshed.dpopKey.jkt,
            exportRejected:
              exportRejected && refreshed.dpopKey.keyPair.privateKey.extractable === false,
            pendingRemoved:
              (await new IndexedDbDpopTransactionStore().load(expected.keyId)) === null,
            transactionRemoved: storage.getTransaction() === null,
            rotated: tokens.refreshToken !== refreshed.refreshToken,
            cookies: document.cookie.length,
          };
        },
        { config, service },
      );
      check(
        result.secure && result.status === 200 && result.userinfo === 401,
        `${service}: API access must succeed and its token must be refused by UserInfo (${result.status}/${result.userinfo})`,
      );
      check(result.downgrade === 401, 'A DPoP token must not become Bearer');
      check(
        result.sameKey &&
          result.exportRejected &&
          result.pendingRemoved &&
          result.transactionRemoved &&
          result.rotated,
        'DPoP key and refresh lifecycle failed',
      );
      check(
        result.subject === config.subject && result.cookies === 0,
        'Client identity or cookie isolation failed',
      );
      sessions.push(page);
      keyThumbprints.add(result.thumbprint);
      check(
        (await context.cookies(clientOrigin)).length === 0,
        'Identity HttpOnly cookies must not reach a client site',
      );
    }
    check(keyThumbprints.size === 2, 'Independent client sites must have independent DPoP keys');
    const cookies = await context.cookies(config.origins.identity);
    const cookie = cookies.find((value) => value.name === '__Host-nvbes-session');
    check(
      cookie?.secure &&
        cookie.httpOnly &&
        cookie.sameSite === 'Lax' &&
        cookie.domain === 'localhost',
      'Identity cookie protections must remain active',
    );
    const attacker = await context.newPage();
    await attacker.goto(config.origins.unregistered);
    const blocked = await attacker.evaluate(async (origins) => {
      try {
        await fetch(`${origins.account}/api/v1/profile`, {
          headers: { authorization: 'Bearer invalid', dpop: 'invalid' },
        });
        return false;
      } catch (error) {
        return error instanceof TypeError;
      }
    }, config.origins);
    check(blocked, 'Browser must enforce CORS for an unregistered origin');
    const identity = await context.newPage();
    await identity.goto(`${config.origins.identity}/__fixture/hosted`);
    const logout = await identity.evaluate(async (csrf) => {
      const { logoutHostedSession } =
        await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
      await logoutHostedSession({ baseUrl: location.origin }, csrf);
      return true;
    }, sessionCsrf);
    check(logout, 'Identity logout must succeed');
    for (const page of sessions) {
      const revoked = await page.evaluate(async () => {
        const { session, endpoint, dpopFetch } = window.fixtureSession;
        const tokens = session.snapshot();
        const response = await dpopFetch(endpoint, {
          key: tokens.dpopKey,
          accessToken: tokens.accessToken,
        });
        let refreshRejected = false;
        try {
          await session.refresh();
        } catch {
          refreshRejected = true;
        }
        return response.status === 401 && refreshRejected && session.snapshot() === null;
      });
      check(revoked, 'Logout must revoke API access and refresh');
    }
    return {
      browser: browser.version(),
      https: true,
      services: ['identity', 'account', 'billing'],
      par: true,
      oidc: true,
      indexedDbDpop: true,
      refresh: true,
      corsEnforced: true,
      logoutRevoked: true,
    };
  } finally {
    await context.close();
  }
}
