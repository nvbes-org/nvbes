import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin audit sorting @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('can prioritize hash anomalies', async ({ page }) => {
    await page.goto('/#audit');
    const section = page.locator('section#audit');

    await expect(section.locator('article').first()).toContainText(
      'internal_admin.workspace.suspend',
    );
    await section.getByRole('combobox').filter({ hasText: 'Recent first' }).click();
    await page.getByRole('option', { name: 'Hash issues' }).click();

    await expect(section.locator('article').first()).toContainText(
      'internal_admin.audit.backfilled',
    );
  });
});
