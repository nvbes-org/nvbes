import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service compliance center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit tenant and user links from compliance rows', async ({ page }) => {
    await page.goto('/#compliance-center');
    const section = page.locator('section#compliance-center');

    await expect(section.getByText('customer@example.test')).toBeVisible();
    await expect(section.getByText('bounce@example.test')).toBeVisible();
    await expect(section.getByText('orphan@example.test')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'customer@example.test' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=compliance_consent/u);

    await page.goto('/#compliance-center');
    await section
      .locator('article')
      .filter({ hasText: 'bounce@example.test' })
      .getByRole('button', { name: 'Open user' })
      .click();
    await expect(page).toHaveURL(/#user-detail/u);
    await expect(page.locator('section#user-detail')).toBeVisible();

    await page.goto('/#compliance-center');
    await section
      .locator('article')
      .filter({ hasText: 'orphan@example.test' })
      .getByRole('button', { name: 'Tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
