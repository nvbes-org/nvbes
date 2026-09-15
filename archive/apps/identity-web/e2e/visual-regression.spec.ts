import { expect, type Page, test } from '@playwright/test';

const visualRoutes = [
  { path: '/login', name: 'login' },
  { path: '/register', name: 'register' },
] as const;

const viewports = [
  { name: 'desktop', width: 1280, height: 800 },
  { name: 'tablet', width: 768, height: 1024 },
  { name: 'mobile', width: 375, height: 667 },
] as const;

const themes = ['light', 'dark'] as const;

test.describe('@visual-regression Identity Public Views', () => {
  for (const viewport of viewports) {
    for (const theme of themes) {
      for (const route of visualRoutes) {
        test(`${route.name} matches pixel baseline on ${viewport.name} in ${theme} theme`, async ({
          page,
        }) => {
          await page.setViewportSize({ width: viewport.width, height: viewport.height });
          await prepareVisualPage(page, route.path, theme);

          await expect(page).toHaveScreenshot(`${route.name}-${viewport.name}-${theme}.png`, {
            animations: 'disabled',
            caret: 'hide',
            maxDiffPixelRatio: 0.002, // Tolerance: 0.2% max difference
            threshold: 0.2,           // Per-pixel color sensitivity
            mask: [
              page.locator('[data-testid="dynamic-timestamp"]'),
              page.locator('[data-testid="csrf-token"]'),
            ],
          });
        });
      }
    }
  }

  test('form validation error state matches visual baseline', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await prepareVisualPage(page, '/register', 'light');

    const emailInput = page.getByRole('textbox', { name: /^Email/u });
    await emailInput.fill('invalid-email-format');
    await emailInput.press('Tab');

    await expect(page.getByRole('alert')).toBeVisible();

    await expect(page).toHaveScreenshot('register-validation-error-state.png', {
      animations: 'disabled',
      caret: 'hide',
      maxDiffPixelRatio: 0.002,
      threshold: 0.2,
    });
  });
});

async function prepareVisualPage(
  page: Page,
  route: string,
  theme: (typeof themes)[number],
) {
  await page.emulateMedia({
    colorScheme: theme,
    reducedMotion: 'reduce',
  });

  await page.goto(route, { waitUntil: 'domcontentloaded' });
  await expect(page.locator('body')).toBeVisible();

  // Apply dark mode class explicitly to html root if dark
  await page.locator('html').evaluate((element, scheme) => {
    element.classList.toggle('dark', scheme === 'dark');
  }, theme);

  // Freeze all CSS transitions, animations, and font-rendering shifts
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation-delay: 0s !important;
        animation-duration: 0.01ms !important;
        transition-delay: 0s !important;
        transition-duration: 0.01ms !important;
      }
      body {
        -webkit-font-smoothing: antialiased;
        -moz-osx-font-smoothing: grayscale;
      }
    `,
  });

  // Ensure DOM has settled
  await page.waitForLoadState('networkidle');
}
