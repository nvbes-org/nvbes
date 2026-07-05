import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin audit saved views @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('persists and reapplies audit filters', async ({ page }) => {
    await page.goto('/#audit');
    const section = page.locator('section#audit');
    const search = section.getByPlaceholder('Search action, actor, target...');

    await search.fill('workspace');
    await section.getByPlaceholder('Nom de vue sauvegardee').fill('Workspace audit');
    await section.getByRole('button', { name: 'Save view' }).click();
    await section.getByRole('button', { name: 'Reset' }).click();
    await expect(search).toHaveValue('');

    await page.reload();
    await section.getByRole('combobox').filter({ hasText: 'Vues sauvegardees' }).click();
    await page.getByRole('option', { name: 'Workspace audit' }).click();

    await expect(search).toHaveValue('workspace');
  });
});
