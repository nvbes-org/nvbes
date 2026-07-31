import { expect, test } from '@playwright/test';

test.describe('@compatibility public security and cookie consent', () => {
  test('document responses expose the browser security policy', async ({ page }) => {
    const response = await page.goto('/login', { waitUntil: 'domcontentloaded' });
    expect(response).not.toBeNull();

    const headers = response?.headers() ?? {};
    expect(headers['content-security-policy']).toContain("default-src 'self'");
    expect(headers['content-security-policy']).not.toContain("'unsafe-eval'");
    expect(headers['x-frame-options']).toBe('DENY');
    expect(headers['referrer-policy']).toBe('strict-origin-when-cross-origin');
    expect(headers['permissions-policy']).toBeTruthy();
  });

  test('declining optional tracking persists a deny decision without analytics cookies', async ({
    context,
    page,
  }) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Tout refuser' }).click();
    await page.waitForLoadState('domcontentloaded');

    const consent = await page.evaluate(() => {
      const raw = window.localStorage.getItem('nvbes.tracking-consent.v4');
      return raw ? JSON.parse(raw) : null;
    });
    expect(consent).not.toBeNull();
    expect(consent.consent.categories.analytics).toBe(false);
    expect(consent.consent.categories.performance).toBe(false);
    expect(consent.consent.vendors.posthog).toBe(false);
    expect(consent.consent.vendors.sentry).toBe(false);

    const cookieNames = (await context.cookies()).map((cookie) => cookie.name.toLowerCase());
    expect(cookieNames.some((name) => name.includes('posthog'))).toBe(false);
    expect(cookieNames.some((name) => name.includes('sentry'))).toBe(false);
  });
});
