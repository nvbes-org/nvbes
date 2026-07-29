import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';

test.describe('@security browser automation defense', () => {
  test('a browser exposing WebDriver is rejected before identifier lookup', async ({
    page,
  }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium', 'Chromium is the anti-automation reference.');

    await page.goto('/login');
    expect(await page.evaluate(() => navigator.webdriver)).toBe(true);
    await page.getByLabel('Email', { exact: true }).fill(`automation-${randomUUID()}@example.test`);
    await page.getByRole('button', { name: 'Continuer' }).click();

    await expect(page.getByText(/vous ne pouvez pas effectuer cette action/iu)).toBeVisible();
    await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
  });
});
