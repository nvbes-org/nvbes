import {
  authorizationFor,
  authenticatorCode,
  confirmHostedAccount,
  prepareHostedFactors,
} from './runtime-browser-hosted-auth.mjs';

export async function verifyHostedSecurity(browser, origin, method) {
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
    await page.goto(origin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    const factors = await prepareHostedFactors(page, config);
    await page.goto(await authorizationFor(page, config));
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    if (await page.getByRole('button', { name: 'Gérer mes méthodes de sécurité' }).count())
      throw new Error('Primary-only login offered factor management');
    await page
      .getByRole('button', { name: 'Confirmer mon identité pour gérer la sécurité' })
      .click();
    await page.getByRole('heading', { name: 'Confirmez votre accès à la sécurité.' }).waitFor();
    await page.getByRole('button', { name: 'Revenir aux accès demandés' }).click();
    await page
      .getByRole('button', { name: 'Confirmer mon identité pour gérer la sécurité' })
      .click();
    const refreshed = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/oauth/authorize/authentication' &&
        response.request().method() === 'POST',
    );
    if (method === 'passkey') {
      await page.getByRole('button', { name: 'Vérifier avec une passkey' }).click();
    } else {
      await page
        .getByLabel('Code à 6 chiffres')
        .fill(
          authenticatorCode(
            factors.secret,
            Math.max(factors.counter + 1, Math.floor(Date.now() / 30_000)),
          ),
        );
      await page.getByRole('button', { name: 'Vérifier le code' }).click();
    }
    const status = await (await refreshed).json();
    if (status.minimum_authentication !== 'primary' || status.proof_expires_at !== null)
      throw new Error('Voluntary MFA changed the registered OAuth policy');
    await page.getByRole('button', { name: 'Gérer mes méthodes de sécurité' }).click();
    await page.getByLabel('Nom de la passkey').waitFor();
    await page.getByRole('button', { name: 'Revenir aux accès demandés' }).click();
    await page.getByRole('button', { name: 'Générer des codes de secours' }).click();
    const codes = page.getByRole('list', { name: 'Codes de secours' });
    await codes.waitFor();
    if ((await codes.getByRole('listitem').count()) !== 10)
      throw new Error('Recovery code display missing');
    await page.getByRole('button', { name: 'J’ai conservé mes codes' }).click();
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    return {
      voluntaryMfa: method,
      oauthPolicyUnchanged: true,
      ...(await confirmHostedAccount(page, config)),
      syntheticAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
