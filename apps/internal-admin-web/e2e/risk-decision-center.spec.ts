import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin risk decision center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit user tenant and workspace links from risk rows', async ({ page }) => {
    await page.goto('/#risk-decision-center');
    const section = page.locator('section#risk-decision-center');

    await expect(section.getByText('risky.user@example.test')).toBeVisible();
    await expect(section.getByText('manual_review')).toBeVisible();
    await expect(section.getByText('provider_dispute_spike')).toBeVisible();
    await expect(section.getByText('Acme HQ')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'risky.user@example.test' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=identity_risk_decision/u);

    await page.goto('/#risk-decision-center');
    await section
      .locator('article')
      .filter({ hasText: 'risky.user@example.test' })
      .getByRole('button', { name: 'User' })
      .click();
    await expect(page).toHaveURL(/#user-detail/u);
    await expect(page.locator('section#user-detail')).toBeVisible();

    await page.goto('/#risk-decision-center');
    await section
      .locator('article')
      .filter({ hasText: 'manual_review' })
      .getByRole('button', { name: 'Tenant' })
      .click();
    await expect(page).toHaveURL(/#tenant-detail/u);
    await expect(page.locator('section#tenant-detail')).toBeVisible();

    await page.goto('/#risk-decision-center');
    await section
      .locator('article')
      .filter({ hasText: 'Acme HQ' })
      .getByRole('button', { name: 'Workspace' })
      .click();
    await expect(page).toHaveURL(/#workspace-detail/u);
    await expect(page.locator('section#workspace-detail')).toBeVisible();
  });
});
