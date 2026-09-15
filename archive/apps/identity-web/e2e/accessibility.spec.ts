import AxeBuilder from '@axe-core/playwright';
import { expect, type Page, test, type TestInfo } from '@playwright/test';

const publicPages = [
  {
    route: '/login',
    heading: 'Se connecter',
    initialField: /^Email$/u,
    nextControl: { kind: 'button', name: 'Continuer' },
  },
  {
    route: '/register',
    heading: 'Créer votre compte',
    initialField: /^Email/u,
    nextControl: { kind: 'button', name: 'Créer mon compte' },
  },
] as const;

const colorSchemes = ['light', 'dark'] as const;
const wcagTags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

test.describe('@accessibility public Identity pages', () => {
  for (const colorScheme of colorSchemes) {
    for (const publicPage of publicPages) {
      test(`${publicPage.route} satisfies WCAG 2.1 & 2.2 AA in ${colorScheme} mode`, async ({
        page,
      }, testInfo) => {
        await openAccessiblePage(page, publicPage.route, colorScheme);
        await expect(
          page.getByRole('heading', { level: 1, name: publicPage.heading }),
        ).toBeVisible();

        const result = await new AxeBuilder({ page }).withTags(wcagTags).analyze();
        // Strict compliance: no critical, serious, or moderate violations
        const violations = result.violations
          .filter(({ impact }) => impact === 'critical' || impact === 'serious' || impact === 'moderate')
          .map(({ help, helpUrl, id, impact, nodes }) => ({
            help,
            helpUrl,
            id,
            impact,
            nodes: nodes.map(({ failureSummary, html, target }) => ({
              failureSummary,
              html,
              target,
            })),
          }));

        await attachAxeDiagnostics(testInfo, result);
        expect(violations).toEqual([]);
      });
    }
  }

  for (const publicPage of publicPages) {
    test(`${publicPage.route} exposes a coherent document, landmarks, and keyboard focus order`, async ({
      page,
    }) => {
      await openAccessiblePage(page, publicPage.route, 'light');

      await expect(page.locator('html')).toHaveAttribute('lang', /^fr(?:-|$)/u);
      await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
      await expect(page.getByRole('heading', { level: 1, name: publicPage.heading })).toBeVisible();
      await expect(page.locator('form')).toHaveCount(1);

      const initialField = page.getByRole('textbox', { name: publicPage.initialField });
      await expect(initialField).toBeVisible();
      await expect(initialField).toHaveAccessibleName(publicPage.initialField);
      await expect(initialField).toBeFocused();

      await page.keyboard.press('Tab');
      if (publicPage.route === '/login') {
        const nextControl = page.getByRole('button', { name: publicPage.nextControl.name });
        await expect(nextControl).toBeFocused();
        await expect(nextControl).toHaveAccessibleName(new RegExp(publicPage.nextControl.name, 'u'));
      }
    });

    test(`${publicPage.route} complies with WCAG 2.2 target size requirements`, async ({ page }) => {
      await openAccessiblePage(page, publicPage.route, 'light');
      const interactiveButtons = page.getByRole('button');
      const buttonCount = await interactiveButtons.count();

      for (let i = 0; i < buttonCount; i++) {
        const button = interactiveButtons.nth(i);
        if (await button.isVisible()) {
          const box = await button.boundingBox();
          if (box) {
            // WCAG 2.2 SC 2.5.8: Target Size (Minimum) >= 24x24 CSS px
            expect(box.height).toBeGreaterThanOrEqual(24);
            expect(box.width).toBeGreaterThanOrEqual(24);
          }
        }
      }
    });
  }

  test('registration validation is announced via ARIA live/alert with zero violations', async ({
    page,
  }, testInfo) => {
    await openAccessiblePage(page, '/register', 'light');
    const email = page.getByRole('textbox', { name: /^Email/u });

    await email.fill('adresse-invalide');
    await email.press('Tab');

    const validationMessage = page
      .getByRole('alert')
      .filter({ hasText: 'Saisissez une adresse email valide.' });
    await expect(email).toHaveAttribute('aria-invalid', 'true');
    await expect(email).toHaveAttribute('aria-describedby', 'register-email-validation');
    await expect(validationMessage).toBeVisible();
    await expect(validationMessage).toHaveAttribute('id', 'register-email-validation');

    const result = await new AxeBuilder({ page }).withTags(wcagTags).analyze();
    const violations = result.violations.filter(
      ({ impact }) => impact === 'critical' || impact === 'serious' || impact === 'moderate',
    );

    await attachAxeDiagnostics(testInfo, result);
    expect(violations).toEqual([]);
  });
});

async function openAccessiblePage(
  page: Page,
  route: string,
  colorScheme: (typeof colorSchemes)[number],
) {
  await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' });
  await page.goto(route, { waitUntil: 'domcontentloaded' });
  await expect(page.locator('body')).toBeVisible();
  await page.locator('html').evaluate((element, scheme) => {
    element.classList.toggle('dark', scheme === 'dark');
  }, colorScheme);
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation-delay: 0s !important;
        animation-duration: 0.01ms !important;
        transition-delay: 0s !important;
        transition-duration: 0.01ms !important;
      }
    `,
  });
  if (colorScheme === 'dark') {
    await expect(page.locator('html')).toHaveClass(/(?:^|\s)dark(?:\s|$)/u);
  } else {
    await expect(page.locator('html')).not.toHaveClass(/(?:^|\s)dark(?:\s|$)/u);
  }
}

async function attachAxeDiagnostics(
  testInfo: TestInfo,
  result: Awaited<ReturnType<AxeBuilder['analyze']>>,
) {
  await testInfo.attach('axe-accessibility-results', {
    body: JSON.stringify(
      {
        incomplete: result.incomplete,
        testEnvironment: result.testEnvironment,
        testRunner: result.testRunner,
        violations: result.violations,
      },
      null,
      2,
    ),
    contentType: 'application/json',
  });
}
