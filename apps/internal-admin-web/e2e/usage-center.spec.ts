import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin usage center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit tenant and workspace links from usage rows', async ({ page }) => {
    await page.goto('/#usage-center');
    const section = page.locator('section#usage-center');

    await expect(section.getByText('storage_gb').first()).toBeVisible();
    await expect(section.getByText('Acme Europe').first()).toBeVisible();
    await expect(section.getByText('api_calls')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'storage_gb' })
      .first()
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=usage_meter/u);

    await page.goto('/#usage-center');
    await section
      .locator('article')
      .filter({ hasText: 'Acme HQ' })
      .getByRole('button', { name: 'Workspace' })
      .click();
    await expect(page).toHaveURL(/#workspace-detail/u);
    await expect(page.locator('section#workspace-detail')).toBeVisible();

    await page.goto('/#usage-center');
    await section
      .locator('article')
      .filter({ hasText: 'api_calls' })
      .getByRole('button', { name: 'Tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
