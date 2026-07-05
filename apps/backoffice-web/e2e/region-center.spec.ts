import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service region center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit workspace and tenant links from region rows', async ({ page }) => {
    await page.goto('/#region-center');
    const section = page.locator('section#region-center');

    await expect(section.getByText('Acme US Analytics')).toBeVisible();
    await expect(section.getByText('Acme Europe', { exact: true })).toBeVisible();
    await expect(section.getByText('CCPA', { exact: true })).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'Acme US Analytics' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=region_workspace/u);

    await page.goto('/#region-center');
    await section
      .locator('article')
      .filter({ hasText: 'Acme US Analytics' })
      .getByRole('button', { name: 'Open workspace' })
      .click();
    await expect(page).toHaveURL(/#workspace-detail/u);
    await expect(page.locator('section#workspace-detail')).toBeVisible();

    await page.goto('/#region-center');
    await section
      .locator('article')
      .filter({ hasText: '2 workspaces across 2 regions' })
      .getByRole('button', { name: 'Open tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
