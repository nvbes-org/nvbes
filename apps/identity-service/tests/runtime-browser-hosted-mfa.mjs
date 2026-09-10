import {
  authorizationFor,
  authenticatorCode,
  confirmHostedAccount,
  logoutHostedUi,
  prepareHostedFactors,
} from './runtime-browser-hosted-auth.mjs';

export async function verifyHostedMfa(browser, clientOrigin) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('WebAuthn.enable');
    await cdp.send('WebAuthn.addVirtualAuthenticator', {
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
    const factors = await prepareHostedFactors(page, config);
    const results = [];
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.name));
    for (const method of ['totp', 'webauthn_step_up', 'passkey_login']) {
      const client = {
        ...config,
        clientId: method === 'totp' ? config.mfaClientId : config.webauthnClientId,
      };
      const authorization = await authorizationFor(page, client);
      await page.goto(authorization);
      await page.getByLabel('Adresse email').waitFor();
      if (method === 'totp') {
        // Emulate a cancelled browser prompt; every HTTP request still uses the real SDK/runtime.
        await page.evaluate(() => {
          const get = navigator.credentials.get.bind(navigator.credentials);
          let cancel = true;
          navigator.credentials.get = (...args) => {
            if (cancel) {
              cancel = false;
              return Promise.reject(new DOMException('Synthetic cancellation', 'NotAllowedError'));
            }
            return get(...args);
          };
        });
        await page.getByRole('button', { name: 'Se connecter avec une passkey' }).click();
        await page.getByRole('alert').waitFor();
        if (!(await page.getByLabel('Adresse email').isVisible()))
          throw new Error('Cancellation must preserve login');
      }
      const loginPath =
        method === 'passkey_login' ? '/oauth/authorize/passkey/finish' : '/oauth/authorize/login';
      const responsePromise = page.waitForResponse(
        (response) =>
          new URL(response.url()).pathname === loginPath && response.request().method() === 'POST',
      );
      if (method === 'passkey_login') {
        await page.getByRole('button', { name: 'Se connecter avec une passkey' }).click();
      } else {
        await page.getByLabel('Adresse email').fill(config.email);
        await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
        await page.getByRole('button', { name: 'Continuer', exact: true }).click();
      }
      const loginResponse = await responsePromise;
      if (loginResponse.status() !== 200) throw new Error('Hosted primary login failed');
      const { session_csrf_token: csrf } = await loginResponse.json();
      if (method !== 'passkey_login') {
        await page.getByRole('heading', { name: 'Confirmez que c’est vous.' }).waitFor();
        if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
          throw new Error('Consent appeared before MFA');
        if (method === 'totp') {
          const code = page.getByLabel('Code à 6 chiffres');
          await code.fill(factors.consumedCode);
          await page.getByRole('button', { name: 'Vérifier le code' }).click();
          await page.getByRole('alert').waitFor();
          if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
            throw new Error('Consumed TOTP bypassed MFA');
          // Use the documented one-step tolerance without changing the server clock.
          const counter = Math.max(factors.counter + 1, Math.floor(Date.now() / 30_000));
          await code.fill(authenticatorCode(factors.secret, counter));
          await page.getByRole('button', { name: 'Vérifier le code' }).click();
        } else {
          if (await page.getByLabel('Code à 6 chiffres').count())
            throw new Error('WebAuthn policy exposed TOTP');
          await page.getByRole('button', { name: 'Vérifier avec une passkey' }).click();
        }
      }
      await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
      await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
      results.push({ method, ...(await confirmHostedAccount(page, client)) });
      await logoutHostedUi(page, config, csrf);
    }
    if (errors.length) throw new Error('Hosted MFA emitted JavaScript errors');
    return {
      builtUiMfa: results,
      cancellationFallback: true,
      consumedTotpRefused: true,
      syntheticAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
