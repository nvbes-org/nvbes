import {
  authorizationFor,
  authenticatorCode,
  confirmHostedAccount,
  prepareHostedFactors,
} from './runtime-browser-hosted-auth.mjs';

export async function verifyHostedFactors(browser, clientOrigin, removed) {
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
    const client = { ...config, clientId: config.mfaClientId };
    await page.goto(await authorizationFor(page, client));
    await page.getByRole('button', { name: 'Se connecter avec une passkey' }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    await page.getByRole('button', { name: 'Gérer mes méthodes de sécurité' }).click();
    await page.getByLabel('Nom de la passkey').fill('Renamed security key');
    await page.getByRole('button', { name: 'Renommer', exact: true }).click();
    await page.getByRole('region', { name: 'Renamed security key' }).waitFor();
    const removeButton = page.getByRole('button', {
      name: removed === 'totp' ? 'Supprimer TOTP' : 'Supprimer cette passkey',
      exact: true,
    });
    await removeButton.click();
    await page.getByRole('button', { name: 'Conserver ce facteur' }).click();
    await removeButton.click();
    await page.getByRole('button', { name: 'Confirmer la suppression' }).click();
    await page.getByText('Le facteur a été supprimé.', { exact: false }).waitFor();
    if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
      throw new Error('Revocation preserved consent');
    await page.goto(new URL(client.redirectUri).origin);
    await page.evaluate(async () => {
      const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
      const { discardAuthorizationRequest } =
        await import('/libs/ts/identity-sdk-web/src/oauth.authorization-cancel.ts');
      await discardAuthorizationRequest({ storage: new BrowserSessionStorage(sessionStorage) });
    });
    await page.goto(await authorizationFor(page, client));
    await page.getByLabel('Adresse email').waitFor();
    if (removed === 'totp') {
      await page.getByRole('button', { name: 'Se connecter avec une passkey' }).click();
    } else {
      await page.getByLabel('Adresse email').fill(config.email);
      await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
      await page.getByRole('button', { name: 'Continuer', exact: true }).click();
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
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    await page.getByRole('button', { name: 'Gérer mes méthodes de sécurité' }).click();
    const last = page.getByRole('button', {
      name: removed === 'totp' ? 'Supprimer cette passkey' : 'Supprimer TOTP',
      exact: true,
    });
    await last.waitFor();
    if (!(await last.isDisabled())) throw new Error('UI offered last-factor removal');
    if (removed === 'totp')
      await page.getByRole('region', { name: 'Renamed security key' }).waitFor();
    await page.getByRole('button', { name: 'Revenir aux accès demandés' }).click();
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    return {
      removed,
      renamed: true,
      lastFactorProtected: true,
      ...(await confirmHostedAccount(page, client)),
      syntheticAuthenticator: true,
    };
  } finally {
    await context.close();
  }
}
