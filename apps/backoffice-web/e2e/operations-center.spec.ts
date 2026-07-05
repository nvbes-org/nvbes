import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service operations center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit and tenant links from operational rows', async ({ page }) => {
    await page.goto('/#operations-center');
    const section = page.locator('section#operations-center');

    await expect(section.getByText('invoice.payment_failed')).toBeVisible();
    await expect(section.getByText('ledger_provider_amount_mismatch')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'invoice.payment_failed' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);

    await page.goto('/#operations-center');
    await section
      .locator('article')
      .filter({ hasText: 'Acme Corp' })
      .first()
      .getByRole('button', { name: 'Open tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
