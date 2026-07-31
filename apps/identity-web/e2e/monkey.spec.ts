import { expect, test } from '@playwright/test';

const expectedOrigin = new URL(process.env.NVBES_WEB_BASE_URL ?? 'http://127.0.0.1:3001').origin;

test.describe('@monkey bounded public auth exploration', () => {
  test('randomized navigation preserves origin and a responsive document', async ({ page }) => {
    test.slow();

    const seed = positiveInteger(process.env.NVBES_MONKEY_SEED, 20_260_729);
    const actions = positiveInteger(process.env.NVBES_MONKEY_ACTIONS, 100);
    const random = xorshift(seed);
    const routes = ['/login', '/register', '/forgot-password'];
    const pageErrors: string[] = [];
    const serverErrors: string[] = [];

    page.on('pageerror', (error) => pageErrors.push(error.message));
    page.on('response', (response) => {
      if (response.status() >= 500) {
        serverErrors.push(`${response.status()} ${response.url()}`);
      }
    });

    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    for (let index = 0; index < actions; index += 1) {
      const action = Math.floor(random() * 5);

      if (action === 0) {
        await page.goto(routes[Math.floor(random() * routes.length)], {
          waitUntil: 'domcontentloaded',
        });
      } else if (action === 1) {
        await page.reload({ waitUntil: 'domcontentloaded' });
      } else if (action === 2) {
        const textbox = page.getByRole('textbox').first();
        if (await textbox.isVisible()) {
          await textbox.fill(`é2e-${index}-${Math.floor(random() * 1_000_000)}@example.test`);
        }
      } else if (action === 3) {
        const links = page.locator(
          'a[href="/login"]:visible, a[href="/register"]:visible, a[href="/forgot-password"]:visible',
        );
        const count = await links.count();
        if (count > 0) {
          await links.nth(Math.floor(random() * count)).click();
        }
      } else {
        const buttons = page.locator('button:visible:not([disabled])');
        const count = await buttons.count();
        if (count > 0) {
          await buttons.nth(Math.floor(random() * count)).click();
        }
      }

      await expect(
        page.locator('body'),
        `iteration=${index} action=${action} url=${page.url()}`,
      ).toBeVisible();
      expect(new URL(page.url()).origin).toBe(expectedOrigin);
    }

    expect(pageErrors).toEqual([]);
    expect(serverErrors).toEqual([]);
  });
});

function positiveInteger(value: string | undefined, fallback: number): number {
  if (value === undefined) {
    return fallback;
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`Expected a positive integer, received ${value}`);
  }
  return parsed;
}

function xorshift(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 4_294_967_296;
  };
}
