import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin audit evidence alerts @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('surfaces evidence and runtime alert rules', async ({ page }) => {
    await page.goto('/#audit-evidence-center');

    const section = page.locator('section#audit-evidence-center');
    await expect(section.getByText('Evidence alerts')).toBeVisible();
    await expect(section.getByText('Audit events without immutable hash')).toBeVisible();
    await expect(section.getByText('Runtime alert rules')).toBeVisible();
    await expect(section.getByText('Back-office mutations are failing')).toBeVisible();
    await expect(section.getByText('internal_admin_action_requests_total')).toBeVisible();
    await expect(section.getByText('Linked')).toBeVisible();
    await expect(section.getByText('Anomalies', { exact: true })).toBeVisible();
    await expect(section.getByText('linked')).toBeVisible();
  });
});
