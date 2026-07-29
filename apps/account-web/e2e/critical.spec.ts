import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { expect, type Page, test, type TestInfo } from '@playwright/test';

import {
  assertBrowserCookieContract,
  csrfRequestHeaders,
  emailTextbox,
  loginThroughBrowser,
  requiredEnvironment,
} from './identity-journey.support';

const repoRoot = path.resolve(process.cwd(), '../..');

// The production bot guard intentionally rejects WebDriver. This browser-only
// launch flag lets the critical user journey exercise the application behind
// that guard; bot rejection itself is covered by bot-defense.spec.ts.
test.use({
  launchOptions: {
    args: ['--disable-blink-features=AutomationControlled'],
  },
  trace: 'off',
  video: 'off',
});

test.describe('@critical identity journeys', () => {
  test.describe.configure({ mode: 'serial' });

  test('registration and email verification work in a real browser', async ({ page }, testInfo) => {
    const { betaEmail, betaPassword, username } = registrationCredentials(testInfo);

    await page.goto('/register');
    await page.getByLabel(/^Prénom/u).fill('Beta');
    await page.getByLabel(/^Nom \*/u).fill('E2E');
    await page.getByLabel(/^Email/u).fill(betaEmail);
    await page.getByLabel(/^Nom d'utilisateur/u).fill(username);
    await page.getByLabel(/^Date de naissance/u).fill('01/01/1990');
    await page.getByLabel(/^Mot de passe/u).fill(betaPassword);
    await page.getByRole('button', { name: 'Continuer vers la dernière étape' }).click();
    await page.getByRole('checkbox', { name: /J'accepte les/u }).check();
    await page.getByRole('button', { name: 'Créer mon compte' }).click();
    await expect(page).toHaveURL(/\/verify\/?$/u);

    const verificationToken = extractEmailToken(betaEmail, 'verification');
    await navigateWithSensitiveToken(page, '/verify-result', verificationToken);
    await expect(page).toHaveURL(/\/verify-result\/?$/u);
    await expect(page.getByText('Email vérifié', { exact: true })).toBeVisible();

    await page.getByRole('button', { name: 'Se connecter' }).click();
    await loginThroughBrowser(page, betaEmail, betaPassword);
  });

  test.describe('seeded account journeys', () => {
    test('login and reset password work in a real browser', async ({ page }, testInfo) => {
      const credentials = betaCredentials(testInfo);
      prepareBetaAccount(credentials);
      const { betaEmail, betaPassword } = credentials;

      await loginThroughBrowser(page, betaEmail, betaPassword);
      await page.goto('/forgot-password');
      await emailTextbox(page).fill(betaEmail);
      await page.getByRole('button', { name: 'Envoyer le lien' }).click();
      await expect(page.getByText(/vous recevrez un email/iu)).toBeVisible();

      const resetToken = extractEmailToken(betaEmail, 'password_reset');
      const nextPassword = `AnotherStrongPassword123!${randomUUID().slice(0, 8)}`;

      await navigateWithSensitiveToken(page, '/reset-password', resetToken);
      await page.getByLabel('Nouveau mot de passe', { exact: true }).fill(nextPassword);
      await page.getByLabel('Confirmer le mot de passe', { exact: true }).fill(nextPassword);
      await page.getByRole('button', { name: 'Reinitialiser le mot de passe' }).click();
      await expect(
        page.getByText('Votre mot de passe a ete reinitialise avec succes.'),
      ).toBeVisible();

      await page.getByRole('button', { name: 'Se connecter' }).click();
      await loginThroughBrowser(page, betaEmail, nextPassword);
    });

    test('session cookies are hardened and logout revokes the browser session', async ({
      page,
    }, testInfo) => {
      const credentials = betaCredentials(testInfo);
      prepareBetaAccount(credentials);
      const { betaEmail, betaPassword } = credentials;

      await loginThroughBrowser(page, betaEmail, betaPassword);
      await assertBrowserCookieContract(page);

      const logoutResponse = await page.request.post('/auth/logout', {
        data: {},
        headers: await csrfRequestHeaders(page),
      });
      expect(logoutResponse.ok(), await logoutResponse.text()).toBe(true);

      await page.goto('/account/0');
      await expect(page).toHaveURL(/\/login(?:\?|$)/u);
    });
  });
});

function prepareBetaAccount(credentials: ReturnType<typeof betaCredentials>) {
  try {
    execFileSync(
      'cargo',
      [
        'run',
        '-q',
        '-p',
        'nvbes-account-service',
        '--',
        '--prepare-beta-e2e-account',
        '--email',
        credentials.betaEmail,
        '--workspace-name',
        process.env.NVBES_BETA_SEED_WORKSPACE ?? 'Beta Staging Workspace',
      ],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          NVBES_ENV: requiredEnvironment('NVBES_TARGET_ENV'),
          NVBES_DATABASE_URL: requiredEnvironment('NVBES_DATABASE_URL'),
          NVBES_REDIS_URL: requiredEnvironment('NVBES_REDIS_URL'),
          NVBES_BETA_SEED_PASSWORD: credentials.betaPassword,
        },
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );
  } catch {
    throw new Error('Beta account preparation failed.');
  }
}

async function navigateWithSensitiveToken(page: Page, pathname: string, token: string) {
  try {
    await page.goto(`${pathname}?token=${encodeURIComponent(token)}`);
  } catch {
    throw new Error(`Sensitive ${pathname} navigation failed.`);
  }
}

function betaCredentials(testInfo: TestInfo) {
  const configuredEmail = requiredEnvironment('NVBES_BETA_SEED_EMAIL');
  const betaPassword = requiredEnvironment('NVBES_BETA_SEED_PASSWORD');
  const separator = configuredEmail.lastIndexOf('@');
  if (separator <= 0) {
    throw new Error('NVBES_BETA_SEED_EMAIL must be a valid email address.');
  }

  const localPart = configuredEmail.slice(0, separator).split('+')[0];
  const domain = configuredEmail.slice(separator + 1);
  const stableSuffix =
    `${testInfo.project.name}-${testInfo.workerIndex}-${testInfo.retry}-${testInfo.title}`
      .toLowerCase()
      .replace(/[^a-z0-9]+/gu, '-')
      .replace(/^-|-$/gu, '')
      .slice(0, 48);

  return {
    betaEmail: `${localPart}+${stableSuffix}@${domain}`,
    betaPassword,
  };
}

function registrationCredentials(testInfo: TestInfo) {
  const credentials = betaCredentials(testInfo);
  const separator = credentials.betaEmail.lastIndexOf('@');
  const nonce = randomUUID().replaceAll('-', '').slice(0, 12);

  return {
    ...credentials,
    betaEmail: `${credentials.betaEmail.slice(0, separator)}-${nonce}${credentials.betaEmail.slice(separator)}`,
    username: `beta_e2e_${nonce}`,
  };
}

function extractEmailToken(email: string, businessType: 'verification' | 'password_reset') {
  const commandOutput = execFileSync(
    'cargo',
    [
      'run',
      '-q',
      '-p',
      'nvbes-account-service',
      '--',
      '--extract-email-token',
      '--email',
      email,
      '--business-type',
      businessType,
    ],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        NVBES_ENV: requiredEnvironment('NVBES_TARGET_ENV'),
        NVBES_DATABASE_URL: requiredEnvironment('NVBES_DATABASE_URL'),
        NVBES_REDIS_URL: requiredEnvironment('NVBES_REDIS_URL'),
      },
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );

  return commandOutput.trim();
}
