import { authenticatorCode } from './runtime-browser-hosted-auth.mjs';

/** Adds the other factor through the built UI after the initial strong proof. */
export async function addHostedFactor(page, context, firstMethod) {
  let authenticator;
  if (firstMethod === 'totp') {
    const cdp = await context.newCDPSession(page);
    await cdp.send('WebAuthn.enable');
    const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'internal',
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    authenticator = { cdp, authenticatorId };
  }
  await page.getByRole('button', { name: 'Ajouter une méthode de sécurité' }).click();
  await page.getByRole('heading', { name: 'Protégez votre compte.' }).waitFor();
  const configure = page.getByRole('button', {
    name: 'Configurer une application d’authentification',
  });
  if (firstMethod === 'totp') {
    if (await configure.count()) throw new Error('Active TOTP offered duplicate enrollment');
    await page.getByLabel('Nom de la passkey').fill('Additional key');
    await page.getByRole('button', { name: 'Créer une passkey' }).click();
  } else {
    await configure.click();
    const key = page.getByLabel('Clé de configuration');
    await key.waitFor();
    const secret = await key.inputValue();
    await page
      .getByLabel('Code de confirmation')
      .fill(authenticatorCode(secret, Math.floor(Date.now() / 30_000)));
    await page.getByRole('button', { name: 'Confirmer l’application' }).click();
    // TOTP confirmation cannot satisfy this client's WebAuthn-only policy.
    await page.getByRole('heading', { name: 'Confirmez que c’est vous.' }).waitFor();
    if (await page.getByRole('button', { name: 'Autoriser et continuer' }).count())
      throw new Error('Additional TOTP bypassed WebAuthn-only policy');
    await page.getByRole('button', { name: 'Vérifier avec une passkey' }).click();
  }
  await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
  return authenticator;
}
