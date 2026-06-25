import { expect, test } from '@playwright/test';
import { installCredentials, installMockApi } from './action-confirmation.mock-api';

test.describe('internal-admin communications center @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('exposes audit links from communication rows', async ({ page }) => {
    await page.goto('/#communications-center');
    const section = page.locator('section#communications-center');

    await expect(section.getByText('invoice_notice', { exact: true })).toBeVisible();
    await expect(section.getByText('billing@example.test').first()).toBeVisible();
    await expect(section.getByText('blocked@example.test')).toBeVisible();
    await expect(section.getByText('evt_email_unprocessed')).toBeVisible();

    await section
      .locator('article')
      .filter({ hasText: 'billing@example.test' })
      .first()
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=email_message/u);

    await page.goto('/#communications-center');
    await section
      .locator('article')
      .filter({ hasText: 'blocked@example.test' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=email_suppression/u);

    await page.goto('/#communications-center');
    await section
      .locator('article')
      .filter({ hasText: 'evt_email_unprocessed' })
      .getByRole('button', { name: 'Audit' })
      .click();
    await expect(page).toHaveURL(/#audit/u);
    await expect(page).toHaveURL(/target_type=email_provider_event/u);
  });
});
