import { expect, type Locator, type Page, test } from '@playwright/test';

const publicRoutes = ['/login', '/register'] as const;

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

  for (const route of publicRoutes) {
    test(`${route} does not overflow the configured viewport`, async ({ page }) => {
      await page.goto(route, { waitUntil: 'domcontentloaded' });
      await expect(page.locator('body')).toBeVisible();
      await expectNoHorizontalOverflow(page);
    });
  }

  test('authentication fields expose stable browser and password-manager semantics', async ({
    page,
  }) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    const loginEmail = page.getByLabel('Email', { exact: true });
    await expectNativeInputContract(loginEmail, {
      autocomplete: 'username webauthn',
      type: 'email',
    });
    await expect(page.getByRole('button', { name: 'Continuer' })).toHaveAttribute('type', 'submit');

    await page.goto('/register', { waitUntil: 'domcontentloaded' });

    const registrationEmail = page.getByLabel(/^Email/u);
    const registrationPassword = page.getByLabel(/^Mot de passe/u);
    await expectNativeInputContract(registrationEmail, {
      autocomplete: 'username',
      type: 'email',
    });
    await expectNativeInputContract(registrationPassword, {
      autocomplete: 'new-password',
      type: 'password',
    });
    await expect(registrationPassword).toHaveAttribute('minlength', '8');
    await expect(page.getByRole('checkbox')).toHaveCount(2);
    await expect(page.getByRole('button', { name: 'Créer mon compte' })).toHaveAttribute(
      'type',
      'submit',
    );
  });

  test('hosted registration contains credentials and consents but no Account profile fields', async ({
    page,
  }) => {
    await page.goto('/register', { waitUntil: 'domcontentloaded' });

    await expect(page.getByLabel(/^Email/u)).toBeVisible();
    await expect(page.getByLabel(/^Mot de passe/u)).toBeVisible();
    await expect(page.getByRole('checkbox', { name: /J'accepte les/u })).toBeVisible();
    await expect(page.locator('input[name="nickname"]')).toHaveCount(0);
    await expect(page.locator('input[name="firstname"]')).toHaveCount(0);
    await expect(page.locator('input[name="lastname"]')).toHaveCount(0);
    await expect(page.locator('input[name="birthdate"]')).toHaveCount(0);
    await expect(page.locator('select[name="region"], input[name="region"]')).toHaveCount(0);
  });

  test('auth links preserve client query parameters in both directions', async ({ page }) => {
    const query = 'compatibility=browser&client_hint=public-app';
    await page.goto(`/login?${query}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByRole('link', { name: "S'inscrire" })).toHaveAttribute(
      'href',
      `/register?${query}`,
    );
    await page.goto(`/register?${query}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByRole('link', { name: 'Se connecter' })).toHaveAttribute(
      'href',
      `/login?${query}`,
    );
  });

  for (const route of publicRoutes) {
    test(`${route} reflows at 200% text size without clipping primary controls`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 640, height: 800 });
      await page.goto(route, { waitUntil: 'domcontentloaded' });
      await page.addStyleTag({ content: 'html { font-size: 200% !important; }' });

      const primaryControl =
        route === '/login'
          ? page.getByRole('button', { name: 'Continuer' })
          : page.getByRole('button', { name: 'Créer mon compte' });
      await expect(primaryControl).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await expectInsideViewport(page, primaryControl);
    });
  }

  test('mobile projects expose touch-sized primary authentication controls', async ({
    page,
  }, testInfo) => {
    test.skip(!testInfo.project.name.startsWith('mobile-'), 'Mobile compatibility contract');

    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    await expectMinimumControlHeight(page.getByLabel('Email', { exact: true }), 44);
    await expectMinimumControlHeight(page.getByRole('button', { name: 'Continuer' }), 44);

    await page.goto('/register', { waitUntil: 'domcontentloaded' });
    await expectMinimumControlHeight(page.getByLabel(/^Email/u), 44);
    await expectMinimumControlHeight(page.getByLabel(/^Mot de passe/u), 44);
    await expectMinimumControlHeight(page.getByRole('button', { name: 'Créer mon compte' }), 44);
  });

  test('reduced motion preference keeps the authentication flow operable', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    await expect(page.getByLabel('Email', { exact: true })).toBeEditable();
    await expect(page.getByRole('button', { name: 'Continuer' })).toBeEnabled();
  });
});

async function expectNativeInputContract(
  input: Locator,
  contract: { autocomplete: string; type: string },
) {
  await expect(input).toBeVisible();
  await expect(input).toHaveAttribute('type', contract.type);
  await expect(input).toHaveAttribute('autocomplete', contract.autocomplete);
  await expect(input).toHaveAttribute('required', '');
}

async function expectNoHorizontalOverflow(page: Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth + 1);
}

async function expectInsideViewport(page: Page, locator: Locator) {
  await locator.scrollIntoViewIfNeeded();
  const bounds = await locator.boundingBox();
  expect(bounds).not.toBeNull();
  const viewport = page.viewportSize();
  expect(viewport).not.toBeNull();
  expect(bounds?.x ?? -1).toBeGreaterThanOrEqual(0);
  expect((bounds?.x ?? 0) + (bounds?.width ?? 0)).toBeLessThanOrEqual(viewport?.width ?? 0);
}

async function expectMinimumControlHeight(locator: Locator, minimum: number) {
  await expect(locator).toBeVisible();
  const cssHeight = await locator.evaluate((element) =>
    Number.parseFloat(globalThis.getComputedStyle(element).height),
  );
  expect(cssHeight).toBeGreaterThanOrEqual(minimum);
}
