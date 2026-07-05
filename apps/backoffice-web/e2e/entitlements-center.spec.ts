import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service entitlements center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit tenant and workspace links from entitlement rows', async ({ page }) => {
    await page.goto('/#entitlements-center');
    const section = page.locator('section#entitlements-center');

    await expect(section.getByText('Drive Scale')).toBeVisible();
    await expect(section.getByText('storage_gb')).toBeVisible();
    await expect(section.getByText('trialing')).toBeVisible();
    await expect(section.getByText('entitlement.catalog.plan.updated')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'Drive Scale' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=entitlement_plan/u);

    await page.goto('/#entitlements-center');
    await section
      .locator('article')
      .filter({ hasText: 'storage_gb' })
      .getByRole('button', { name: 'Workspace' })
      .click();
    await expect(page).toHaveURL(/#workspace-detail/u);
    await expect(page.locator('section#workspace-detail')).toBeVisible();

    await page.goto('/#entitlements-center');
    await section
      .locator('article')
      .filter({ hasText: 'entitlement.catalog.plan.updated' })
      .getByRole('button', { name: 'Tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();
  });
});
