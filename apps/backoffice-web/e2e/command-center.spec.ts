import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('backoffice-service command center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test("renders today's work with operational links", async ({ page }) => {
    await page.goto('/#command-center');
    const section = page.locator('section#command-center');

    await expect(section.getByText("Today's work")).toBeVisible();
    await expect(section.getByRole('link', { name: /Pending approvals/i })).toBeVisible();
    await expect(section.getByRole('link', { name: /Open incidents/i })).toBeVisible();
    await expect(section.getByRole('link', { name: /Audit anomalies/i })).toBeVisible();
    await expect(section.getByRole('link', { name: /SLA pressure/i })).toBeVisible();

    await section.getByRole('link', { name: /Pending approvals/i }).click();
    await expect(page).toHaveURL(/#pending-approvals/u);
    await expect(page.locator('section#pending-approvals')).toBeVisible();
  });
});
