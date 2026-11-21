import { authorizationFor, confirmHostedAccount } from './runtime-browser-hosted-auth.mjs';

/** Consumes a code displayed by the real UI; no recovery mutation is sent by the test SDK. */
export async function verifyRecoveryFromUi(page, context, client, code) {
  await page.goto(await authorizationFor(page, client));
  await page.getByLabel('Adresse email').fill(client.email);
  await page.getByRole('button', { name: 'Continuer', exact: true }).click();
  await page.getByLabel('Mot de passe', { exact: true }).fill(client.password);
  await page.getByRole('button', { name: 'Se connecter', exact: true }).click();
  await page.getByRole('heading', { name: 'Confirmez que c’est vous.' }).waitFor();
  await page.getByText('Vous avez perdu vos facteurs ?', { exact: true }).click();
  await page.getByLabel('Code de secours', { exact: true }).fill(code);
  await page.getByRole('button', { name: 'Utiliser ce code de secours' }).click();
  await page.waitForURL(client.origins.identity + '/recovery');
  await page.getByRole('heading', { name: 'Remplacez votre facteur perdu.' }).waitFor();
  await page.reload();
  await page.getByRole('heading', { name: 'Remplacez votre facteur perdu.' }).waitFor();
  if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
    throw new Error('Recovery exposed ordinary consent');
  const cdp = await context.newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'usb',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  await page.evaluate(() => {
    const create = navigator.credentials.create.bind(navigator.credentials);
    let cancelled = false;
    navigator.credentials.create = (...args) => {
      if (!cancelled) {
        cancelled = true;
        return Promise.reject(new DOMException('Synthetic cancellation', 'NotAllowedError'));
      }
      return create(...args);
    };
  });
  await page.getByRole('button', { name: 'Créer la passkey de remplacement' }).click();
  await page.getByRole('alert').waitFor();
  await page.getByLabel('Nom de la nouvelle passkey').fill('Replacement browser key');
  await page.getByRole('button', { name: 'Créer la passkey de remplacement' }).click();
  await page.getByRole('heading', { name: 'Votre nouvelle passkey est prête.' }).waitFor();
  if (!(await page.evaluate(() => localStorage.length === 0 && sessionStorage.length === 0)))
    throw new Error('Recovery persisted browser storage');
  await page.goto(new URL(client.redirectUri).origin);
  await page.evaluate(async () => {
    const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
    const { discardAuthorizationRequest } =
      await import('/libs/ts/identity-sdk-web/src/oauth.authorization-cancel.ts');
    await discardAuthorizationRequest({ storage: new BrowserSessionStorage(sessionStorage) });
  });
  await page.goto(await authorizationFor(page, client));
  await page.getByLabel('Adresse email').waitFor();
  await page.getByRole('button', { name: 'Se connecter avec une passkey' }).click();
  await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
  await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
  return {
    ...(await confirmHostedAccount(page, client)),
    freshLogin: true,
    reload: true,
    cancellation: true,
  };
}
