import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin developer center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit and tenant links from developer rows', async ({ page }) => {
    await page.goto('/#developer-center');
    const section = page.locator('section#developer-center');

    await expect(section.getByText('Acme Marketplace Sync').first()).toBeVisible();
    await expect(section.getByText('billing-events-endpoint')).toBeVisible();
    await expect(section.getByText('Billing write access')).toBeVisible();
    await expect(section.getByText('webhook_signature_validation')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'Acme Marketplace Sync' })
      .first()
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=marketplace_app/u);

    await page.goto('/#developer-center');
    await section
      .locator('article')
      .filter({ hasText: 'billing-events-endpoint' })
      .getByRole('button', { name: 'Tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();

    await page.goto('/#developer-center');
    await section
      .locator('article')
      .filter({ hasText: 'Billing write access' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=developer_scope/u);
  });
});
