import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { expect, type Page, test, type TestInfo } from '@playwright/test';

import {
  assertBrowserCookieContract,
  csrfRequestHeaders,
  loginThroughBrowser,
  requiredEnvironment,
  submitPrimaryCredentials,
} from './identity-journey.support';
import { completeHostedAuthorizationCodeJourney } from './oauth-journey.support';

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
    const { betaEmail, betaPassword } = registrationCredentials(testInfo);

    await page.goto('/register');
    await page.getByLabel(/^Email/u).fill(betaEmail);
    await page.getByLabel(/^Mot de passe/u).fill(betaPassword);
    await expect(page.getByLabel(/^Nom d'utilisateur/u)).toHaveCount(0);
    await page.getByRole('checkbox', { name: /J'accepte les/u }).check();
    await page.getByRole('button', { name: 'Créer mon compte' }).click();
    await expect(page).toHaveURL(/\/verify\/?$/u);

    const verificationToken = extractEmailToken(betaEmail, 'verification');
    await navigateWithSensitiveToken(page, '/verify-result', verificationToken);
    await expect(page).toHaveURL(/\/verify-result\/?$/u);
    await expect(page.getByText('Email vérifié', { exact: true })).toBeVisible();

    await navigateWithSensitiveToken(page, '/verify-result', `${verificationToken}-tampered`);
    await expect(page.getByText('Lien invalide', { exact: true })).toBeVisible();

    await loginThroughBrowser(page, betaEmail, betaPassword);
  });

  test.describe('seeded account journeys', () => {
    test('invalid credentials never create an authenticated browser session', async ({
      page,
    }, testInfo) => {
      const credentials = betaCredentials(testInfo);
      prepareBetaAccount(credentials);

      await submitPrimaryCredentials(
        page,
        credentials.betaEmail,
        `${credentials.betaPassword}-invalid`,
      );

      await expect(page.getByLabel('Mot de passe', { exact: true })).toBeVisible();
      await expectUnauthenticated(page);
    });

    test('session cookies are hardened and logout revokes the browser session', async ({
      page,
    }, testInfo) => {
      const credentials = betaCredentials(testInfo);
      prepareBetaAccount(credentials);
      const { betaEmail, betaPassword } = credentials;

      await loginThroughBrowser(page, betaEmail, betaPassword);
      await assertBrowserCookieContract(page);
      await expectAuthenticatedAs(page, betaEmail);

      const logoutResponse = await page.request.post('/auth/logout', {
        data: {},
        headers: await csrfRequestHeaders(page),
      });
      expect(logoutResponse.ok(), await logoutResponse.text()).toBe(true);
      await expectUnauthenticated(page);

      await page.goto('/login');
      await expect(page.getByRole('heading', { name: 'Se connecter' })).toBeVisible();
    });

    test('hosted OAuth authorization enforces PAR, consent and PKCE', async ({
      page,
    }, testInfo) => {
      const credentials = betaCredentials(testInfo);
      prepareBetaAccount(credentials);

      await completeHostedAuthorizationCodeJourney(
        page,
        credentials.betaEmail,
        credentials.betaPassword,
      );
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
        'nvbes-identity-service',
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
  const nonce = randomUUID().replaceAll('-', '').slice(0, 8);
  const localPart = credentials.betaEmail.slice(0, separator);
  const boundedLocalPart = localPart.slice(0, 64 - nonce.length - 1);

  return {
    ...credentials,
    betaEmail: `${boundedLocalPart}-${nonce}${credentials.betaEmail.slice(separator)}`,
  };
}

async function expectAuthenticatedAs(page: Page, email: string) {
  const response = await page.request.get('/auth/accounts', { failOnStatusCode: false });
  const responseBody = await response.text();
  expect(response.status(), responseBody).toBe(200);

  const payload: unknown = JSON.parse(responseBody);
  expect(payload).toMatchObject({ accounts: [{ user: { email } }] });
}

async function expectUnauthenticated(page: Page) {
  const response = await page.request.get('/auth/accounts', { failOnStatusCode: false });
  const responseBody = await response.text();
  expect(response.status(), responseBody).toBe(200);
  expect(JSON.parse(responseBody)).toEqual({ accounts: [] });

  const cookieNames = (await page.context().cookies()).map((cookie) => cookie.name);
  expect(cookieNames).not.toContain('session');
  expect(cookieNames).not.toContain('__Host-session');
}

function extractEmailToken(email: string, businessType: 'verification') {
  const commandOutput = execFileSync(
    'cargo',
    [
      'run',
      '-q',
      '-p',
      'nvbes-identity-service',
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
