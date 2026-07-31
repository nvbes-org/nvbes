import { expect, test } from '@playwright/test';

test.describe('@compatibility public account experience', () => {
  test('login remains usable with accessible names and focus', async ({
    browserName,
    page,
  }, testInfo) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    const email = page.getByLabel('Email', { exact: true });
    const continueButton = page.getByRole('button', { name: 'Continuer' });

    await expect(email).toBeVisible();
    await expect(continueButton).toBeVisible();
    await expect(email).toHaveAccessibleName('Email');
    await expect(continueButton).toHaveAccessibleName('Continuer');

    await email.focus();
    await expect(email).toBeFocused();
    await page.keyboard.type('browser-compatibility@example.test');
    if (browserName === 'webkit' || testInfo.project.name.startsWith('mobile-')) {
      expect(await continueButton.evaluate((element) => element.tabIndex)).toBeGreaterThanOrEqual(
        0,
      );
      await continueButton.focus();
    } else {
      await page.keyboard.press('Tab');
    }
    await expect(continueButton).toBeFocused();
  });

  for (const route of ['/login', '/register', '/forgot-password']) {
    test(`${route} does not overflow the configured viewport`, async ({ page }) => {
      await page.goto(route, { waitUntil: 'domcontentloaded' });
      await expect(page.locator('body')).toBeVisible();

      const dimensions = await page.evaluate(() => ({
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
      }));
      expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth + 1);
    });
  }

  test('reduced motion preference keeps the authentication flow operable', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    await expect(page.getByLabel('Email', { exact: true })).toBeEditable();
    await expect(page.getByRole('button', { name: 'Continuer' })).toBeEnabled();
  });
});
