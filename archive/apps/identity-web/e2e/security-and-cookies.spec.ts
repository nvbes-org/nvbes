import { expect, type Page, test } from '@playwright/test';
import { installAccountConsentSwitchScenario } from './security-and-cookies.support';

const CONSENT_STORAGE_KEY = 'nvbes.tracking-consent.v4';
const CONSENT_TTL_DAYS = 183;
const OPTIONAL_TRACKING_RESOURCE = /(?:posthog|sentry|grafana|analytics-posthog)/iu;

interface StoredConsent {
  version: number;
  savedAt: string;
  expiresAt: string;
  source: string;
  consent: {
    categories: Record<string, boolean>;
    vendors: Record<string, boolean>;
    analytics: Record<string, boolean>;
  };
}

test.describe('@compatibility public security and cookie consent', () => {
  test('document responses expose the browser security policy', async ({ page }) => {
    const response = await page.goto('/login', { waitUntil: 'domcontentloaded' });
    expect(response).not.toBeNull();

    const headers = response?.headers() ?? {};
    const contentSecurityPolicy = requiredHeader(headers, 'content-security-policy');

    for (const directive of [
      "default-src 'self'",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
      "frame-ancestors 'none'",
      "script-src-attr 'none'",
      "require-trusted-types-for 'script'",
    ]) {
      expect(contentSecurityPolicy).toContain(directive);
    }
    assertUnsafeEvalIsLocalDevelopmentOnly(contentSecurityPolicy, response?.url() ?? page.url());
    expect(headers['x-frame-options']).toBe('DENY');
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['referrer-policy']).toBe('strict-origin-when-cross-origin');
    expect(headers['cross-origin-embedder-policy']).toBe('require-corp');
    expect(headers['cross-origin-opener-policy']).toBe('same-origin');
    expect(headers['cross-origin-resource-policy']).toBe('same-origin');
    expect(requiredHeader(headers, 'permissions-policy')).toContain('camera=()');
    expect(requiredHeader(headers, 'permissions-policy')).toContain('microphone=()');

    const metaPolicy = await page
      .locator('meta[http-equiv="Content-Security-Policy"]')
      .getAttribute('content');
    expect(metaPolicy).toContain("default-src 'self'");
    expect(metaPolicy).toContain("object-src 'none'");
    assertUnsafeEvalIsLocalDevelopmentOnly(metaPolicy ?? '', response?.url() ?? page.url());
  });

  test('HTTPS documents publish a preload-grade transport policy', async ({ page }) => {
    const response = await page.goto('/login', { waitUntil: 'domcontentloaded' });
    expect(response).not.toBeNull();

    const responseUrl = new URL(response?.url() ?? page.url());
    test.skip(responseUrl.protocol !== 'https:', 'HSTS is meaningful only over HTTPS.');

    const transportPolicy = requiredHeader(response?.headers() ?? {}, 'strict-transport-security');
    expect(transportPolicy).toContain('max-age=63072000');
    expect(transportPolicy).toContain('includeSubDomains');
    expect(transportPolicy).toContain('preload');
  });

  test('declining optional tracking persists a deny decision without analytics cookies', async ({
    context,
    page,
  }) => {
    const optionalTrackingRequests: string[] = [];
    page.on('request', (request) => {
      if (OPTIONAL_TRACKING_RESOURCE.test(request.url())) {
        optionalTrackingRequests.push(request.url());
      }
    });

    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      page.getByRole('button', { name: 'Tout refuser' }).click(),
    ]);

    const storedConsent = await readStoredConsent(page);
    expect(storedConsent.version).toBe(4);
    expect(storedConsent.source).toBe('identity-web:banner:decline-all');
    expect(storedConsent.consent.categories).toMatchObject({
      essentials: true,
      analytics: false,
      performance: false,
    });
    expect(storedConsent.consent.vendors).toMatchObject({
      stripe: true,
      identity: true,
      cloudflare: true,
      posthog: false,
      sentry: false,
      grafana: false,
    });
    expect(Object.values(storedConsent.consent.analytics).every((value) => value === false)).toBe(
      true,
    );

    const savedAt = Date.parse(storedConsent.savedAt);
    const expiresAt = Date.parse(storedConsent.expiresAt);
    expect(Number.isFinite(savedAt)).toBe(true);
    expect(Number.isFinite(expiresAt)).toBe(true);
    const nominalTtl = CONSENT_TTL_DAYS * 24 * 60 * 60 * 1_000;
    const daylightSavingTolerance = 60 * 60 * 1_000;
    expect(expiresAt - savedAt).toBeGreaterThanOrEqual(nominalTtl - daylightSavingTolerance);
    expect(expiresAt - savedAt).toBeLessThanOrEqual(nominalTtl + daylightSavingTolerance);

    const cookieNames = (await context.cookies()).map((cookie) => cookie.name.toLowerCase());
    expect(cookieNames.some((name) => OPTIONAL_TRACKING_RESOURCE.test(name))).toBe(false);
    const applicationOrigin = new URL(page.url()).origin;
    const outboundTrackingRequests = optionalTrackingRequests.filter(
      (requestUrl) => new URL(requestUrl).origin !== applicationOrigin,
    );
    expect(outboundTrackingRequests).toEqual([]);

    await expect(page.getByRole('button', { name: 'Tout refuser' })).toHaveCount(0);
  });

  test('corrupted consent is discarded and never suppresses the privacy choice', async ({
    page,
  }) => {
    await page.addInitScript((storageKey) => {
      window.localStorage.setItem(storageKey, '{not-valid-json');
    }, CONSENT_STORAGE_KEY);

    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    await expect(page.getByRole('button', { name: 'Tout refuser' })).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate((storageKey) => window.localStorage.getItem(storageKey), CONSENT_STORAGE_KEY),
      )
      .toBeNull();
  });

  test('expired consent is discarded and asks for a fresh decision', async ({ page }) => {
    await page.addInitScript(
      ({ storageKey, expiredAt }) => {
        window.localStorage.setItem(
          storageKey,
          JSON.stringify({
            version: 4,
            savedAt: expiredAt,
            expiresAt: expiredAt,
            source: 'expired-e2e-fixture',
            consent: {
              categories: { essentials: true, analytics: false, performance: false },
              vendors: {
                stripe: true,
                identity: true,
                cloudflare: true,
                posthog: false,
                sentry: false,
                grafana: false,
              },
              analytics: {},
            },
          }),
        );
      },
      {
        storageKey: CONSENT_STORAGE_KEY,
        expiredAt: '2020-01-01T00:00:00.000Z',
      },
    );

    await page.goto('/login', { waitUntil: 'domcontentloaded' });

    await expect(page.getByRole('button', { name: 'Tout refuser' })).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate((storageKey) => window.localStorage.getItem(storageKey), CONSENT_STORAGE_KEY),
      )
      .toBeNull();
  });

  test('switching users disables previous cookies then restores the target consent', async ({
    context,
    page,
  }) => {
    await page.addInitScript(
      ({ consent, storageKey }) => {
        if (window.localStorage.getItem(storageKey) !== null) {
          return;
        }

        const now = new Date();
        const expiresAt = new Date(now);
        expiresAt.setDate(expiresAt.getDate() + 183);
        window.localStorage.setItem(
          storageKey,
          JSON.stringify({
            version: 4,
            savedAt: now.toISOString(),
            expiresAt: expiresAt.toISOString(),
            source: 'previous-account',
            consent,
          }),
        );
        document.cookie = 'ph_previous_user=previous-user; path=/; SameSite=Lax';
      },
      { consent: acceptedConsent(), storageKey: CONSENT_STORAGE_KEY },
    );
    const scenario = await installAccountConsentSwitchScenario(page);

    await page.goto('/login?return_to=/switch-complete', { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: /Compte B/u }).click();

    await expect
      .poll(() => readStoredConsent(page))
      .toMatchObject({
        source: 'identity-web:account-switch',
        consent: { categories: { analytics: false, performance: false } },
      });
    await expect
      .poll(async () => (await context.cookies()).some((cookie) => cookie.name.startsWith('ph_')))
      .toBe(false);

    scenario.releaseTargetConsent();
    await expect.poll(() => new URL(page.url()).pathname).toBe('/switch-complete');
    await expect
      .poll(() => readStoredConsent(page))
      .toMatchObject({
        source: 'tracking-consent:sync:backend',
        consent: {
          categories: { analytics: true, performance: false },
          vendors: { posthog: true, sentry: false, grafana: false },
        },
      });
  });
});

function requiredHeader(headers: Record<string, string>, name: string): string {
  const value = headers[name];
  expect(value, `response header ${name}`).toBeTruthy();
  return value ?? '';
}

function assertUnsafeEvalIsLocalDevelopmentOnly(policy: string, responseUrl: string): void {
  if (!policy.includes("'unsafe-eval'")) {
    return;
  }

  expect(['127.0.0.1', 'localhost', '[::1]']).toContain(new URL(responseUrl).hostname);
}

async function readStoredConsent(page: Page): Promise<StoredConsent> {
  const raw = await page.evaluate(
    (storageKey) => window.localStorage.getItem(storageKey),
    CONSENT_STORAGE_KEY,
  );
  expect(raw, 'tracking consent must be persisted').not.toBeNull();
  return JSON.parse(raw ?? 'null') as StoredConsent;
}

function acceptedConsent() {
  return {
    categories: { essentials: true, analytics: true, performance: true },
    vendors: {
      stripe: true,
      identity: true,
      cloudflare: true,
      posthog: true,
      sentry: true,
      grafana: true,
    },
    analytics: {
      productAnalytics: true,
      autocaptureHeatmaps: false,
      sessionReplay: false,
      surveysFeedback: false,
      errorTracking: true,
      featureFlags: false,
    },
  };
}
