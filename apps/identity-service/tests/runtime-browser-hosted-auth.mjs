import { createHmac } from 'node:crypto';

export async function authorizationFor(page, config) {
  await page.goto(new URL(config.redirectUri).origin);
  return page.evaluate(async (config) => {
    const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
    const { createAuthorizationRequest } =
      await import('/libs/ts/identity-sdk-web/src/oauth.authorization-request.ts');
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
}

/** A synthetic authenticator only; Identity verifies every submitted code. */
export function authenticatorCode(secret, counter) {
  const bytes = [];
  let accumulator = 0;
  let bits = 0;
  for (const character of secret) {
    accumulator = (accumulator << 5) | 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'.indexOf(character);
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((accumulator >>> bits) & 255);
    }
  }
  const moving = Buffer.alloc(8);
  moving.writeBigUInt64BE(BigInt(counter));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(moving).digest();
  return ((digest.readUInt32BE(digest.at(-1) & 15) & 0x7fffffff) % 1_000_000)
    .toString()
    .padStart(6, '0');
}

/** Seed factors through active SDK ceremonies; does not stand in for an enrollment screen. */
export async function prepareHostedFactors(page, config) {
  const authorization = await authorizationFor(page, config);
  await page.goto(config.origins.identity + '/__fixture/hosted');
  const prepared = await page.evaluate(
    async ({ config, authorization }) => {
      const { loadHostedAuthorization, loginHostedPassword } =
        await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
      const { registerHostedPasskey, stepUpHostedPasskey } =
        await import('/libs/ts/identity-sdk-web/src/hosted.webauthn.ts');
      const { startHostedTotpEnrollment } =
        await import('/libs/ts/identity-sdk-web/src/hosted.totp.ts');
      const transport = { baseUrl: location.origin };
      const interaction = await loadHostedAuthorization(transport, authorization);
      const state = await loginHostedPassword(transport, interaction, {
        email: config.email,
        password: config.password,
      });
      await registerHostedPasskey(transport, state.sessionCsrfToken, 'Hosted UI verification key');
      await stepUpHostedPasskey(transport, state.sessionCsrfToken);
      return {
        state,
        csrf: state.sessionCsrfToken,
        enrollment: await startHostedTotpEnrollment(transport, state.sessionCsrfToken),
      };
    },
    { config, authorization },
  );
  const counter = Math.floor(Date.now() / 30_000);
  const code = authenticatorCode(prepared.enrollment.secretBase32, counter);
  const redirect = await page.evaluate(
    async ({ prepared, code }) => {
      const { confirmHostedTotpEnrollment } =
        await import('/libs/ts/identity-sdk-web/src/hosted.totp.ts');
      const { completeHostedConsent } =
        await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
      const transport = { baseUrl: location.origin };
      await confirmHostedTotpEnrollment(
        transport,
        prepared.csrf,
        prepared.enrollment.factorId,
        code,
      );
      return completeHostedConsent(transport, prepared.state, 'approve');
    },
    { prepared, code },
  );
  await page.goto(redirect);
  await confirmHostedAccount(page, config);
  await logoutHostedUi(page, config, prepared.csrf);
  return { secret: prepared.enrollment.secretBase32, consumedCode: code, counter };
}

export async function confirmHostedAccount(page, config) {
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
      accountStatus: response.status,
      subjectMatches: tokens.identity.subject === config.subject,
    };
  }, config);
  if (result.accountStatus !== 200 || !result.subjectMatches)
    throw new Error('Strong hosted login did not authorize Account');
  return result;
}

export async function logoutHostedUi(page, config, csrf) {
  await page.goto(config.origins.identity + '/__fixture/hosted');
  await page.evaluate(async (csrf) => {
    const { logoutHostedSession } = await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
    await logoutHostedSession({ baseUrl: location.origin }, csrf);
  }, csrf);
}
