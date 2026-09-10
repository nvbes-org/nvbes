// Real HTTPS services and SDK. WebCrypto emulates only the synthetic authenticator.
export async function verifyBrowserTotp(browser, clientOrigin) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    const authorizationUrl = await page.evaluate(async (config) => {
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
    const response = await page.goto(authorizationUrl);
    if (response.status() !== 200) throw new Error('Authorization failed');
    const interaction = await response.json();
    await page.goto(`${config.origins.identity}/__fixture/hosted`);
    const redirect = await page.evaluate(
      async ({ config, interaction }) => {
        const { parseHostedInteraction, loginHostedPassword, completeHostedConsent } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        const { NvbesIdentityWeb } =
          await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
        const { HostedIdentityError } =
          await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
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
        const enrollment = await client.startTotpEnrollment(state.sessionCsrfToken);
        if (Date.parse(enrollment.expiresAt) <= Date.now()) throw new Error('Enrollment expired');
        const bytes = [];
        let accumulator = 0;
        let bits = 0;
        for (const character of enrollment.secretBase32) {
          accumulator = (accumulator << 5) | 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'.indexOf(character);
          bits += 5;
          if (bits >= 8) {
            bits -= 8;
            bytes.push((accumulator >>> bits) & 255);
          }
        }
        const key = await crypto.subtle.importKey(
          'raw',
          new Uint8Array(bytes),
          { name: 'HMAC', hash: 'SHA-1' },
          false,
          ['sign'],
        );
        const totp = async (counter) => {
          const moving = new ArrayBuffer(8);
          new DataView(moving).setBigUint64(0, BigInt(counter));
          const digest = new Uint8Array(await crypto.subtle.sign('HMAC', key, moving));
          const offset = digest[digest.length - 1] & 15;
          const binary = new DataView(digest.buffer).getUint32(offset) & 0x7fffffff;
          return (binary % 1_000_000).toString().padStart(6, '0');
        };
        const first = await totp(Math.floor(Date.now() / 30_000));
        const expiry = await client.confirmTotpEnrollment(
          state.sessionCsrfToken,
          enrollment.factorId,
          first,
        );
        if (Date.parse(expiry) <= Date.now()) throw new Error('Confirmation did not grant step-up');
        const expectReplay = async (code) => {
          try {
            await client.stepUpTotp(state.sessionCsrfToken, code);
          } catch (error) {
            if (error instanceof HostedIdentityError && error.status === 400) return;
            throw error;
          }
          throw new Error('Consumed TOTP accepted twice');
        };
        await expectReplay(first);
        // Exercise the documented +1 tolerance without a wall-clock sleep or server clock change.
        const next = await totp(Math.floor(Date.now() / 30_000) + 1);
        const nextExpiry = await client.stepUpTotp(state.sessionCsrfToken, next);
        if (Date.parse(nextExpiry) <= Date.now()) throw new Error('Fresh step-up failed');
        await expectReplay(next);
        return completeHostedConsent(transport, state, 'approve');
      },
      { config, interaction },
    );
    await page.goto(redirect);
    const result = await page.evaluate(async (config) => {
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
    if (result.status !== 200 || result.subject !== config.subject)
      throw new Error('Account identity mismatch');
    return {
      browser: browser.version(),
      enrollment: true,
      confirmation: true,
      stepUp: true,
      confirmationCodeReplayRefused: true,
      stepUpCodeReplayRefused: true,
      oidcAndAccount: true,
      syntheticAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
