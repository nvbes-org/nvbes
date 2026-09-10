import {
  authorizationFor,
  authenticatorCode,
  confirmHostedAccount,
} from './runtime-browser-hosted-auth.mjs';

/** Fresh fixture per method: neither factor is seeded through the SDK. */
export async function verifyHostedEnrollment(browser, clientOrigin, method) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.name));
    if (method === 'passkey') {
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
    }
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    const client = {
      ...config,
      clientId: method === 'totp' ? config.mfaClientId : config.webauthnClientId,
    };
    await page.goto(await authorizationFor(page, client));
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('heading', { name: 'Protégez votre compte.' }).waitFor();
    if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
      throw new Error('Consent appeared before first-factor confirmation');
    if (method === 'passkey') {
      if (
        await page
          .getByRole('button', {
            name: 'Configurer une application d’authentification',
          })
          .count()
      )
        throw new Error('WebAuthn policy offered TOTP enrollment');
      await page.getByLabel('Nom de la passkey').fill('Browser enrollment test');
      await page.getByRole('button', { name: 'Créer une passkey' }).click();
    } else {
      const configure = page.getByRole('button', {
        name: 'Configurer une application d’authentification',
      });
      await configure.click();
      const key = page.getByLabel('Clé de configuration');
      await key.waitFor();
      const abandoned = await key.inputValue();
      await page.getByRole('button', { name: 'Autre méthode' }).click();
      await configure.click();
      await key.waitFor();
      const secret = await key.inputValue();
      if (secret === abandoned) throw new Error('Restart reused abandoned TOTP secret');
      await page
        .getByLabel('Code de confirmation')
        .fill(authenticatorCode(secret, Math.floor(Date.now() / 30_000)));
      await page.getByRole('button', { name: 'Confirmer l’application' }).click();
    }
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    if (await page.getByLabel('Clé de configuration').count())
      throw new Error('Provisioning secret remained in consent DOM');
    const storageEmpty = await page.evaluate(
      () => localStorage.length === 0 && sessionStorage.length === 0,
    );
    if (!storageEmpty) throw new Error('Identity enrollment persisted browser storage');
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    const result = await confirmHostedAccount(page, client);
    if (errors.length) throw new Error('Enrollment emitted JavaScript errors');
    return {
      builtUiEnrollment: method,
      ...result,
      syntheticAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
