import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service revenue center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit and tenant links from revenue rows', async ({ page }) => {
    await page.goto('/#revenue-center');
    const section = page.locator('section#revenue-center');

    await expect(section.getByText('INV-2026-0001')).toBeVisible();
    await expect(section.getByText('needs_response')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'INV-2026-0001' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);

    await page.goto('/#revenue-center');
    await section
      .locator('article')
      .filter({ hasText: 'escalated' })
      .getByRole('button', { name: 'Open tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
