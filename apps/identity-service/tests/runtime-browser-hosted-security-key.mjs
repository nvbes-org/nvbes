import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import {
  authorizationFor,
  confirmHostedAccount,
  logoutHostedUi,
} from './runtime-browser-hosted-auth.mjs';

/** A USB CTAP2 device without resident storage; no credential/options rewriting. */
export async function verifySecurityKey(clientOrigin) {
  const { chromium } = createRequire(resolve('apps/identity-web/package.json'))('playwright');
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.name));
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
    const client = { ...config, clientId: config.webauthnClientId };
    let assertions = 0;
    let lastAssertion;
    page.on('request', (request) => {
      if (new URL(request.url()).pathname === '/oauth/session/webauthn/step-up/finish') {
        assertions++;
        lastAssertion = {
          body: request.postDataJSON(),
          csrf: request.headers()['x-csrf-token'],
          url: request.url(),
        };
      }
    });
    for (const first of [true, false]) {
      await page.goto(await authorizationFor(page, client));
      await page.getByLabel('Adresse email').fill(config.email);
      await page.getByRole('button', { name: 'Continuer', exact: true }).click();
      await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
      const login = page.waitForResponse(
        (response) => new URL(response.url()).pathname === '/oauth/authorize/login',
      );
      await page.getByRole('button', { name: 'Se connecter', exact: true }).click();
      const { session_csrf_token: csrf } = await (await login).json();
      await page
        .getByRole('heading', {
          name: first ? 'Protégez votre compte.' : 'Confirmez que c’est vous.',
          exact: true,
        })
        .waitFor();
      if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
        throw new Error('Password alone satisfied WebAuthn policy');
      if (first) {
        await page.getByLabel('Nom de la passkey').fill('Clé USB de test');
        await page.getByRole('button', { name: 'Créer une passkey' }).click();
      } else {
        await page.screenshot({
          path: '/tmp/nvbes-security-key-desktop.png',
          animations: 'disabled',
        });
        await page.setViewportSize({ width: 390, height: 844 });
        if (
          await page.evaluate(
            () =>
              document.documentElement.scrollWidth > innerWidth ||
              [...document.querySelectorAll('button, p')].some((element) => {
                const rect = element.getBoundingClientRect();
                return rect.right > innerWidth || rect.left < 0;
              }),
          )
        )
          throw new Error('Security-key mobile layout overflows');
        await page.screenshot({
          path: '/tmp/nvbes-security-key-mobile.png',
          animations: 'disabled',
          fullPage: true,
        });
        const optionsResponse = page
          .waitForResponse(
            (response) =>
              new URL(response.url()).pathname === '/oauth/session/webauthn/step-up/options',
          )
          .then((response) => response.json());
        await page
          .getByRole('button', { name: 'Vérifier avec une passkey ou une clé de sécurité' })
          .click();
        const options = await optionsResponse;
        const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
        const expectedId = Buffer.from(credentials[0].credentialId, 'base64').toString('base64url');
        if (
          options.options.publicKey.allowCredentials.length !== 1 ||
          options.options.publicKey.allowCredentials[0].id !== expectedId ||
          options.options.publicKey.userVerification !== 'required'
        )
          throw new Error('Security-key options were not bound to the enrolled credential');
      }
      await page.getByRole('heading', { name: 'Accès demandés', exact: true }).waitFor();
      const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
      if (
        credentials.length !== 1 ||
        credentials[0].isResidentCredential ||
        credentials[0].signCount < (first ? 1 : 2)
      )
        throw new Error('Expected a non-resident key with a verified assertion');
      if (!lastAssertion || lastAssertion.body.credential.response.userHandle != null)
        throw new Error('Non-resident assertion unexpectedly required a user handle');
      const replayStatus = await page.evaluate(async ({ body, csrf, url }) => {
        const response = await fetch(url, {
          method: 'POST',
          headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
          body: JSON.stringify(body),
        });
        return response.status;
      }, lastAssertion);
      if (replayStatus !== 400) throw new Error('Replayed security-key assertion was accepted');
      await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
      await confirmHostedAccount(page, client);
      if (first) await logoutHostedUi(page, config, csrf);
    }
    if (errors.length) throw new Error('Security-key UI emitted JavaScript errors');
    return {
      browser: browser.version(),
      builtUi: true,
      virtualUsbKey: true,
      residentCredential: false,
      firstEnrollment: true,
      subsequentPasswordAndKeyLogin: true,
      userHandleRequired: false,
      replayRefused: true,
      accountStatus: 200,
      mobileOverflow: false,
      assertionSubmissions: assertions,
    };
  } finally {
    await browser.close();
  }
}
