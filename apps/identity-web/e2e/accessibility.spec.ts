import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

const publicRoutes = ['/login', '/register', '/forgot-password'];
const colorSchemes = ['light', 'dark'] as const;

test.describe('@accessibility public Account pages', () => {
  for (const colorScheme of colorSchemes) {
    for (const route of publicRoutes) {
      test(`${route} has no serious or critical automated violations in ${colorScheme} mode`, async ({
        page,
      }) => {
        await page.emulateMedia({ reducedMotion: 'reduce' });
        await page.goto(route, { waitUntil: 'domcontentloaded' });
        await expect(page.getByRole('heading').first()).toBeVisible();

        if (colorScheme === 'dark') {
          await page.locator('html').evaluate((element) => element.classList.add('dark'));
        }

        await page.waitForFunction(() =>
          document.getAnimations().every((animation) => animation.playState === 'finished'),
        );

        const result = await new AxeBuilder({ page }).analyze();
        const blockingViolations = result.violations
          .filter((violation) => violation.impact === 'critical' || violation.impact === 'serious')
          .map((violation) => ({
            help: violation.help,
            id: violation.id,
            impact: violation.impact,
            nodes: violation.nodes.map((node) => node.target.join(' ')),
          }));

        expect(blockingViolations).toEqual([]);
      });
    }
  }
});
