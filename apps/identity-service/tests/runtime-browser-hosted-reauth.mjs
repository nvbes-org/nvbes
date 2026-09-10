import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import {
  authorizationFor,
  authenticatorCode,
  confirmHostedAccount,
} from './runtime-browser-hosted-auth.mjs';

/** Runs inside the owning fixture so aging never needs an HTTP mutation endpoint. */
export async function verifyHostedReauthentication(fixture, origin) {
  const { chromium } = createRequire(resolve('apps/identity-web/package.json'))('playwright');
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    const page = await context.newPage();
    await page.goto(origin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    await page.goto(await authorizationFor(page, config));
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    await fixture.sql(
      'identity',
      "UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '6 minutes' WHERE revoked_at IS NULL",
    );
    await page.getByRole('button', { name: 'Ajouter une méthode de sécurité' }).click();
    const refused = page.waitForResponse(
      (response) => new URL(response.url()).pathname === '/oauth/session/totp/enrollment/start',
    );
    await page
      .getByRole('button', { name: 'Configurer une application d’authentification' })
      .click();
    if ((await refused).status() !== 400) throw new Error('Old primary session enrolled a factor');
    await page.getByRole('alert').waitFor();
    await page.getByRole('button', { name: 'Ajouter une méthode de sécurité' }).click();
    await page.getByRole('button', { name: 'Me reconnecter avant la configuration' }).click();
    await page.getByRole('heading', { name: 'Reconnectez-vous pour continuer.' }).waitFor();
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill('Wrong-synthetic-password!');
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('alert').waitFor();
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByLabel('Mot de passe', { exact: true }).fill(config.password);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    await page.getByRole('button', { name: 'Ajouter une méthode de sécurité' }).click();
    await page
      .getByRole('button', { name: 'Configurer une application d’authentification' })
      .click();
    const key = page.getByLabel('Clé de configuration');
    await key.waitFor();
    const secret = await key.inputValue();
    await page
      .getByLabel('Code de confirmation')
      .fill(authenticatorCode(secret, Math.floor(Date.now() / 30_000)));
    await page.getByRole('button', { name: 'Confirmer l’application' }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    await page.getByRole('button', { name: 'Autoriser et continuer' }).click();
    return {
      browser: browser.version(),
      oldSessionRefused: true,
      primaryReauthentication: true,
      ...(await confirmHostedAccount(page, config)),
    };
  } finally {
    await browser.close();
  }
}
