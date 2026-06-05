import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { expect, type Page, test } from '@playwright/test';

const databaseURL = process.env.NVBES_DATABASE_URL;

if (!databaseURL) {
  throw new Error('NVBES_DATABASE_URL is required.');
}

const repoRoot = path.resolve(process.cwd(), '../..');

test.describe('critical identity journeys', () => {
  test('verification, login, logout, and reset password work in a real browser', async ({
    page,
  }) => {
    const betaEmail = process.env.NVBES_BETA_SEED_EMAIL;
    const betaPassword = process.env.NVBES_BETA_SEED_PASSWORD;

    if (!betaEmail || !betaPassword) {
      throw new Error('NVBES_BETA_SEED_EMAIL and NVBES_BETA_SEED_PASSWORD are required.');
    }

    await loginThroughBrowser(page, betaEmail, betaPassword);
    await expect(page).toHaveURL(/\/account$/);

    await logoutThroughBrowser(page);
    await expect(page).toHaveURL(/\/login$/);

    await page.goto('/forgot-password');
    await page.getByLabel('Email').fill(betaEmail);
    await page.getByRole('button', { name: 'Envoyer le lien' }).click();
    await expect(page.getByText(/vous recevrez un email/i)).toBeVisible();

    const resetToken = await extractEmailToken(betaEmail, 'password_reset');
    const nextPassword = 'AnotherStrongPassword123!';

    await page.goto(`/reset-password?token=${resetToken}`);
    await page.getByLabel('Nouveau mot de passe').fill(nextPassword);
    await page.getByLabel('Confirmer le mot de passe').fill(nextPassword);
    await page.getByRole('button', { name: 'Reinitialiser le mot de passe' }).click();
    await expect(
      page.getByText('Votre mot de passe a ete reinitialise avec succes.'),
    ).toBeVisible();

    await page.getByRole('button', { name: 'Se connecter' }).click();
    await loginThroughBrowser(page, betaEmail, nextPassword);
    await expect(page).toHaveURL(/\/account$/);
  });

  test('service accounts admin surface is hidden for non admin workspaces', async ({ page }) => {
    const betaEmail = process.env.NVBES_BETA_SEED_EMAIL;
    const betaPassword = process.env.NVBES_BETA_SEED_PASSWORD;

    if (!betaEmail || !betaPassword) {
      throw new Error('NVBES_BETA_SEED_EMAIL and NVBES_BETA_SEED_PASSWORD are required.');
    }

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
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Continuer' }).click();
  await expect(page.getByLabel('Mot de passe')).toBeVisible();
  await page.getByLabel('Mot de passe').fill(password);
  await page.getByRole('button', { name: 'Se connecter' }).click();
  await expect(page.getByRole('button', { name: 'Se deconnecter' })).toBeVisible();
}

async function logoutThroughBrowser(page: Page) {
  await Promise.all([
    page.waitForURL(/\/login$/),
    page.getByRole('button', { name: 'Se deconnecter' }).first().click(),
  ]);
}

async function extractEmailToken(email: string, businessType: 'verification' | 'password_reset') {
  const commandOutput = execFileSync(
    'cargo',
    [
      'run',
      '-q',
      '-p',
      'nvbes-identity-api',
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
  const response = await page.request.post(`/auth/workspaces/${workspaceId}/switch`);

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
