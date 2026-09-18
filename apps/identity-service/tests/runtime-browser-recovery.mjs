// Real HTTPS services and SDK; only CTAP2 authenticators are virtual.
export async function verifyBrowserRecovery(browser, clientOrigin) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const clientPage = await context.newPage();
    const identityPage = await context.newPage();
    await clientPage.goto(clientOrigin);
    const config = await clientPage.evaluate(async () => (await fetch('/__fixture/config')).json());
    const cdp = await context.newCDPSession(identityPage);
    await cdp.send('WebAuthn.enable');
    const authenticatorOptions = {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    };
    const original = await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: authenticatorOptions,
    });
    const begin = async () => {
      const url = await clientPage.evaluate(async (config) => {
        const { createAuthorizationRequest } =
          await import('/libs/ts/identity-sdk-web/src/oauth.authorization-request.ts');
        const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
        return (
          await createAuthorizationRequest(
            {
              baseUrl: config.origins.identity,
              clientId: config.clientId,
              redirectUri: config.redirectUri,
              resource: config.origins.account,
              storage: new BrowserSessionStorage(sessionStorage),
            },
            { scope: 'openid account:read' },
          )
        ).authorizationUrl;
      }, config);
      const response = await identityPage.goto(url);
      if (response.status() !== 200) throw new Error('Authorization failed');
      const interaction = await response.json();
      await identityPage.goto(`${config.origins.identity}/__fixture/hosted`);
      return interaction;
    };
    const exchange = async (redirect) => {
      await clientPage.goto(redirect);
      const result = await clientPage.evaluate(async (config) => {
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
        const endpoint = `${config.origins.account}/api/v1/profile`;
        window.fixtureRecoveryAccess = { tokens, dpopFetch, endpoint };
        return {
          status: (
            await dpopFetch(endpoint, { key: tokens.dpopKey, accessToken: tokens.accessToken })
          ).status,
          subject: tokens.identity.subject,
        };
      }, config);
      if (result.status !== 200 || result.subject !== config.subject)
        throw new Error('Account identity mismatch');
    };
    const interaction = await begin();
    const redirect = await identityPage.evaluate(
      async ({ config, interaction }) => {
        const { NvbesIdentityWeb } =
          await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
        const { parseHostedInteraction, loginHostedPassword, completeHostedConsent } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        const client = new NvbesIdentityWeb({
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          resource: config.origins.account,
        });
        const transport = { baseUrl: config.origins.identity };
        const state = await loginHostedPassword(transport, parseHostedInteraction(interaction), {
          email: config.email,
          password: config.password,
        });
        const oldKey = await client.registerPasskey(state.sessionCsrfToken, 'Original key');
        await client.stepUpPasskey(state.sessionCsrfToken);
        const codes = await client.generateRecoveryCodes(state.sessionCsrfToken);
        // Secrets stay in this page's ephemeral memory, never tool output or browser storage.
        window.fixtureRecovery = { client, codes, state, oldKey };
        return completeHostedConsent(transport, state, 'approve');
      },
      { config, interaction },
    );
    await exchange(redirect);
    await cdp.send('WebAuthn.removeVirtualAuthenticator', {
      authenticatorId: original.authenticatorId,
    });
    await identityPage.evaluate(async () => {
      const { client, state, codes } = window.fixtureRecovery;
      window.fixtureRecovery.recovery = await client.redeemRecoveryCode(
        state.sessionCsrfToken,
        codes[0],
      );
      const { HostedIdentityError } =
        await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
      try {
        await client.listPasskeys(state.sessionCsrfToken);
      } catch (error) {
        if (error instanceof HostedIdentityError && error.status === 403) return;
        throw error;
      }
      throw new Error('Recovery must remove ordinary session authority');
    });
    const recoveryCookies = await context.cookies(config.origins.identity);
    const cookie = recoveryCookies.find((value) => value.name === '__Host-nvbes-recovery');
    if (
      !cookie?.secure ||
      !cookie.httpOnly ||
      cookie.path !== '/' ||
      cookie.sameSite !== 'Lax' ||
      recoveryCookies.some((value) => value.name === '__Host-nvbes-session')
    )
      throw new Error('Recovery cookie isolation failed');
    const rejected = await clientPage.evaluate(async () => {
      const { tokens, dpopFetch, endpoint } = window.fixtureRecoveryAccess;
      return (await dpopFetch(endpoint, { key: tokens.dpopKey, accessToken: tokens.accessToken }))
        .status;
    });
    if (rejected !== 401) throw new Error('Recovery must revoke previously issued Account access');
    await cdp.send('WebAuthn.addVirtualAuthenticator', { options: authenticatorOptions });
    const beforeReload = await identityPage.evaluate(() => ({
      oldKey: window.fixtureRecovery.oldKey,
      expiresAt: window.fixtureRecovery.recovery.expiresAt,
    }));
    await identityPage.reload();
    await identityPage.evaluate(
      async ({ config, beforeReload }) => {
        if (window.fixtureRecovery) throw new Error('Reload must discard in-memory recovery state');
        const { NvbesIdentityWeb } =
          await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
        const client = new NvbesIdentityWeb({
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          resource: config.origins.account,
        });
        const recovery = await client.resumeMfaRecovery();
        if (recovery.expiresAt !== beforeReload.expiresAt)
          throw new Error('Resume extended recovery expiry');
        window.fixtureRecovery = { client, recovery, oldKey: beforeReload.oldKey };
      },
      { config, beforeReload },
    );
    await identityPage.evaluate(async () => {
      const { client, recovery } = window.fixtureRecovery;
      const { WebauthnBrowserError } =
        await import('/libs/ts/identity-sdk-web/src/webauthn.types.ts');
      const controller = new AbortController();
      controller.abort();
      let cancelled = false;
      try {
        await client.completeMfaRecovery(recovery, 'Cancelled replacement', {
          signal: controller.signal,
        });
      } catch (error) {
        cancelled =
          error instanceof WebauthnBrowserError &&
          error.cause instanceof DOMException &&
          error.cause.name === 'AbortError';
      }
      if (!cancelled) throw new Error('Cancelled ceremony unexpectedly completed');
      const result = await client.completeMfaRecovery(recovery, 'Recovered key');
      if (!result.mustReauthenticate) throw new Error('Recovery bypassed reauthentication');
      window.fixtureRecovery.newKey = result.credentialId;
    });
    const cookies = await context.cookies(config.origins.identity);
    if (
      cookies.some((value) =>
        ['__Host-nvbes-session', '__Host-nvbes-recovery'].includes(value.name),
      )
    )
      throw new Error('Recovery completion retained authentication cookies');
    // Preserve only non-secret identifiers across navigation; codes and proof are discarded.
    const keys = await identityPage.evaluate(() => ({
      old: window.fixtureRecovery.oldKey,
      current: window.fixtureRecovery.newKey,
    }));
    const next = await begin();
    const finalRedirect = await identityPage.evaluate(
      async ({ config, next, keys }) => {
        const { NvbesIdentityWeb } =
          await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
        const { parseHostedInteraction, completeHostedConsent } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        const client = new NvbesIdentityWeb({
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          resource: config.origins.account,
        });
        const pending = parseHostedInteraction(next);
        if (!pending.needsLogin) throw new Error('Recovery must require a new login');
        const state = await client.loginPasskey(pending);
        const factors = await client.listPasskeys(state.sessionCsrfToken);
        if (
          factors.length !== 1 ||
          factors[0].id !== keys.current ||
          factors.some((factor) => factor.id === keys.old)
        )
          throw new Error('Old key survived recovery or replacement is missing');
        const codes = await client.generateRecoveryCodes(state.sessionCsrfToken);
        window.fixtureRecoveryCancellation = { client, codes, csrf: state.sessionCsrfToken };
        return completeHostedConsent({ baseUrl: config.origins.identity }, state, 'approve');
      },
      { config, next, keys },
    );
    await exchange(finalRedirect);
    await identityPage.evaluate(async () => {
      const { client, codes, csrf } = window.fixtureRecoveryCancellation;
      const recovery = await client.redeemRecoveryCode(csrf, codes[0]);
      await client.cancelMfaRecovery(recovery);
      const { HostedIdentityError } =
        await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
      try {
        await client.resumeMfaRecovery();
      } catch (error) {
        if (error instanceof HostedIdentityError && error.status === 400) return;
        throw error;
      }
      throw new Error('Cancelled recovery was resumed');
    });
    const afterCancel = await context.cookies(config.origins.identity);
    if (
      afterCancel.some((value) =>
        ['__Host-nvbes-session', '__Host-nvbes-recovery'].includes(value.name),
      )
    )
      throw new Error('Cancellation retained or restored an authentication cookie');
    const cancelledAccess = await clientPage.evaluate(async () => {
      const { tokens, dpopFetch, endpoint } = window.fixtureRecoveryAccess;
      return (await dpopFetch(endpoint, { key: tokens.dpopKey, accessToken: tokens.accessToken }))
        .status;
    });
    if (cancelledAccess !== 401) throw new Error('Cancellation restored revoked Account access');
    return {
      browser: browser.version(),
      syntheticAuthenticators: true,
      codeRedemption: true,
      separateRecoveryCookie: true,
      oldAccountAccessRevoked: true,
      nativeCancellation: true,
      replacement: true,
      freshPasskeyLogin: true,
      oldFactorRevoked: true,
      newRecoveryCodes: true,
      reloadWithoutExtendingExpiry: true,
      explicitCancellation: true,
      oidcAndAccount: true,
    };
  } finally {
    await context.close();
  }
}
