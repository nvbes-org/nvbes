import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin billing platform center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit and tenant links from billing platform rows', async ({ page }) => {
    await page.goto('/#billing-platform-center');
    const section = page.locator('section#billing-platform-center');

    await expect(section.getByText('stripe', { exact: true })).toBeVisible();
    await expect(section.getByText('Acme Europe SAS')).toBeVisible();
    await expect(section.getByText('fr-chorus-pro')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: '10 - stripe' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=billing_provider_routing_rule/u);

    await page.goto('/#billing-platform-center');
    await section
      .locator('article')
      .filter({ hasText: 'Acme Europe SAS' })
      .getByRole('button', { name: 'Open tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
