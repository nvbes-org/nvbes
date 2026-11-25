import { createDecipheriv, randomBytes } from 'node:crypto';
import { chromium } from 'playwright';

/** Reads only the encrypted synthetic fixture queue, never a provider mailbox. */
export async function verifyPasswordRecovery(fixture, config, key) {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    viewport: { width: 1280, height: 800 },
  });
  let stage = 'request';
  try {
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.name));
    await page.goto(config.origins.identity + '/password-recovery');
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByRole('button', { name: 'Envoyer le lien' }).click();
    await page
      .getByText('Si cette adresse correspond à un compte éligible,', { exact: false })
      .waitFor();
    stage = 'queue';
    if (!/^[0-9a-f-]{36}$/i.test(config.subject)) throw new Error('Invalid fixture subject');
    const row = JSON.parse(
      await fixture.sql(
        'identity',
        `SELECT json_build_object('id',challenge_id,'principal',principal_id,'cipher',encode(ciphertext,'hex'),'nonce',encode(nonce,'hex')) FROM identity_recovery_deliveries WHERE principal_id='${config.subject}'`,
      ),
    );
    const ciphertext = Buffer.from(row.cipher, 'hex');
    const decipher = createDecipheriv('aes-256-gcm', key, Buffer.from(row.nonce, 'hex'));
    decipher.setAAD(
      Buffer.from(`nvbes-identity:v1:${row.principal}:${row.id}:password-recovery-email`),
    );
    decipher.setAuthTag(ciphertext.subarray(-16));
    const command = JSON.parse(
      Buffer.concat([decipher.update(ciphertext.subarray(0, -16)), decipher.final()]).toString(),
    );
    const link = new URL(command.template.PasswordResetV1.reset_url);
    if (
      link.origin !== config.origins.identity ||
      link.pathname !== '/password-recovery' ||
      link.search
    )
      throw new Error('Invalid fixture link');
    const token = new URLSearchParams(link.hash.slice(1)).get('token');
    if (!token) throw new Error('Missing fixture token');
    let leaked = false;
    page.on('request', (request) => {
      if (request.url().includes(token) || (request.headers().referer ?? '').includes(token))
        leaked = true;
    });
    stage = 'reset';
    await page.goto(link.href);
    await page.getByLabel('Nouveau mot de passe', { exact: true }).waitFor();
    if (new URL(page.url()).hash || new URL(page.url()).search)
      throw new Error('Link not scrubbed');
    await page.screenshot({
      path: '/tmp/nvbes-password-recovery-desktop.png',
      fullPage: true,
      animations: 'disabled',
    });
    await page.setViewportSize({ width: 390, height: 844 });
    if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth))
      throw new Error('Mobile overflow');
    await page.screenshot({
      path: '/tmp/nvbes-password-recovery-mobile.png',
      fullPage: true,
      animations: 'disabled',
    });
    const password = 'Browser-recovered-' + randomBytes(20).toString('hex');
    await page.getByLabel('Nouveau mot de passe', { exact: true }).fill(password);
    await page.getByLabel('Confirmer le mot de passe').fill(password);
    await page.getByRole('button', { name: 'Changer le mot de passe' }).click();
    await page.getByRole('heading', { name: 'Mot de passe modifié' }).waitFor();
    const retained = await page.evaluate(
      ({ token, password }) =>
        JSON.stringify([localStorage, sessionStorage, document.body.innerText]).includes(token) ||
        JSON.stringify([localStorage, sessionStorage, document.body.innerText]).includes(password),
      { token, password },
    );
    if (retained || leaked || errors.length) throw new Error('Secret retention or page error');
    stage = 'login';
    await page.goto(config.origins.client);
    const authorization = await page.evaluate(async (config) => {
      const { createAuthorizationRequest, BrowserSessionStorage } =
        await import('/libs/ts/identity-sdk-web/src/oauth.ts');
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
    await page.goto(authorization);
    await page.getByLabel('Adresse email').fill(config.email);
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page.getByLabel('Mot de passe', { exact: true }).fill(password);
    await page.getByRole('button', { name: 'Se connecter', exact: true }).click();
    await page.getByRole('heading', { name: 'Accès demandés' }).waitFor();
    stage = 'replay';
    await page.goto(link.href);
    await page.getByLabel('Nouveau mot de passe', { exact: true }).fill(password + '-again');
    await page.getByLabel('Confirmer le mot de passe').fill(password + '-again');
    await page.getByRole('button', { name: 'Changer le mot de passe' }).click();
    await page.getByRole('alert').waitFor();
    return {
      browser: browser.version(),
      passwordRecovery: true,
      encryptedQueueFixture: true,
      linkScrubbed: true,
      mobileOverflow: false,
      newPasswordLogin: true,
      replayRefused: true,
      secretLeak: false,
    };
  } catch {
    throw new Error(
      `Password recovery browser verification failed at ${stage}; sensitive details suppressed`,
    );
  } finally {
    await context.close();
    await browser.close();
  }
}
