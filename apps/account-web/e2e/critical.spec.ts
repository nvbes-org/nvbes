import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { expect, type Page, test } from '@playwright/test';

const databaseURL = process.env.NVBES_DATABASE_URL;

if (!databaseURL) {
  throw new Error('NVBES_DATABASE_URL is required.');
}

const repoRoot = path.resolve(process.cwd(), '../..');

test.describe('critical identity journeys', () => {
  test.beforeEach(() => {
    const { betaEmail, betaPassword } = betaCredentials();

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
        betaEmail,
        '--password',
        betaPassword,
        '--workspace-name',
        process.env.NVBES_BETA_SEED_WORKSPACE ?? 'Beta Staging Workspace',
      ],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          NVBES_ENV: 'staging',
          NVBES_DATABASE_URL: databaseURL,
        },
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );
  });

  test('verification, login, and reset password work in a real browser', async ({ page }) => {
    const { betaEmail, betaPassword } = betaCredentials();

    await loginThroughBrowser(page, betaEmail, betaPassword);
    await expect(page).toHaveURL(/\/account$/);

    await page.goto('/forgot-password');
    await emailTextbox(page).fill(betaEmail);
    await page.getByRole('button', { name: 'Envoyer le lien' }).click();
    await expect(page.getByText(/vous recevrez un email/i)).toBeVisible();

    const resetToken = await extractEmailToken(betaEmail, 'password_reset');
    const nextPassword = `AnotherStrongPassword123!${randomUUID().slice(0, 8)}`;

    await page.goto(`/reset-password?token=${resetToken}`);
    await page.getByLabel('Nouveau mot de passe', { exact: true }).fill(nextPassword);
    await page.getByLabel('Confirmer le mot de passe', { exact: true }).fill(nextPassword);
    await page.getByRole('button', { name: 'Reinitialiser le mot de passe' }).click();
    await expect(
      page.getByText('Votre mot de passe a ete reinitialise avec succes.'),
    ).toBeVisible();

    await page.getByRole('button', { name: 'Se connecter' }).click();
    await loginThroughBrowser(page, betaEmail, nextPassword);
    await expect(page).toHaveURL(/\/account$/);
  });

  test('service accounts admin surface is hidden for non admin workspaces', async ({ page }) => {
    const { betaEmail, betaPassword } = betaCredentials();

    await loginThroughBrowser(page, betaEmail, betaPassword);

    const workspaces = await listCurrentWorkspaces(page);
    const memberWorkspace = workspaces.find(
      (workspace) => workspace.role !== 'owner' && workspace.role !== 'admin',
    );

    if (!memberWorkspace) {
      throw new Error('No non-admin workspace is available to validate the hidden gate.');
    }

    await switchWorkspace(page, memberWorkspace.id);

    await page.goto('/account');
    await expect(page.getByRole('link', { name: 'Comptes de service' })).toHaveCount(0);

    await page.goto('/account/workspaces/service-accounts');
    await expect(page.getByText('Acces restreint aux comptes de service')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Comptes de service' })).toHaveCount(0);
  });
});

async function loginThroughBrowser(page: Page, email: string, password: string) {
  await page.goto('/login');
  await emailTextbox(page).fill(email);
  await page.getByRole('button', { name: 'Continuer' }).click();
  await expect(page.getByLabel('Mot de passe', { exact: true })).toBeVisible();
  await page.getByLabel('Mot de passe', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Se connecter' }).click();
  await expect(page).toHaveURL(/\/account$/);
}

function emailTextbox(page: Page) {
  return page.getByLabel('Email', { exact: true });
}

function betaCredentials() {
  const betaEmail = process.env.NVBES_BETA_SEED_EMAIL;
  const betaPassword = process.env.NVBES_BETA_SEED_PASSWORD;

  if (!betaEmail || !betaPassword) {
    throw new Error('NVBES_BETA_SEED_EMAIL and NVBES_BETA_SEED_PASSWORD are required.');
  }

  return { betaEmail, betaPassword };
}

async function extractEmailToken(email: string, businessType: 'verification' | 'password_reset') {
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
        NVBES_ENV: 'staging',
        NVBES_DATABASE_URL: databaseURL,
      },
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );

  return commandOutput.trim();
}

async function switchWorkspace(page: Page, workspaceId: string) {
  const response = await page.request.post(`/auth/workspaces/${workspaceId}/switch`, {
    data: {},
    headers: await csrfRequestHeaders(page),
  });

  if (!response.ok()) {
    throw new Error(`Failed to switch workspace: ${response.status()} ${await response.text()}`);
  }
}

async function listCurrentWorkspaces(page: Page) {
  const response = await page.request.get('/workspaces');

  if (!response.ok()) {
    throw new Error(`Failed to load workspaces: ${response.status()} ${await response.text()}`);
  }

  const payload = (await response.json()) as {
    workspaces?: Array<{ id: string; role: string }>;
  };

  return payload.workspaces ?? [];
}

async function csrfRequestHeaders(page: Page) {
  const cookies = await page.context().cookies();
  const csrfCookie = cookies.find((cookie) => cookie.name.endsWith('csrf_token'));

  if (!csrfCookie) {
    throw new Error('CSRF cookie is required for authenticated mutating requests.');
  }

  return {
    Origin: new URL(page.url()).origin,
    'X-CSRF-Token': csrfCookie.value,
  };
}
