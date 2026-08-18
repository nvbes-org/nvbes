import { chromium, expect, type Request, type Response, test } from '@playwright/test';

import {
  CLIENT_PROBE_ATTACKS,
  FULL_STEALTH_INIT_SCRIPT,
  REQUEST_EVASION_ATTACKS,
} from './bot-defense.attack-profiles';
import { BOT_GUARD_ATTACKS, POW_ATTACKS } from './bot-defense.protocol-profiles';
import {
  expectBlockedResponseHasNoAuthenticationMaterial,
  expectClientProbeAttack,
  expectForwardedIpRotationRateLimitedSoft,
  expectHardenedBlockedResponse,
  expectIdentifierBlockedSoft,
  expectNoAuthenticatedSession,
  expectRateLimitHeaders,
  IDENTIFIER_PATH,
  jsonObject,
  PASSWORD_PATH,
  requestPath,
  requireReferenceBrowser,
  responseErrorCode,
  rewriteIdentifierRequest,
  submitIdentifier,
  submitIdentifierAfterRateLimitReset,
  submitIdentifierLikeHuman,
  uniqueEmail,
  waitForRateLimitReset,
} from './bot-defense.support';

/**
 * Black-box adversarial contract: attack profiles use only browser and HTTP behavior observable by
 * a hostile client. TLS/JA4, ASN reputation, residential-proxy rotation and real-device farms live
 * below Playwright's application boundary and require dedicated edge/infrastructure tests.
 */

test.describe('@security browser automation defense', () => {
  test.describe.configure({ mode: 'serial', timeout: 180_000 });

  test('a browser exposing WebDriver is rejected before identifier lookup', async ({
    page,
  }, testInfo) => {
    requireReferenceBrowser(testInfo.project.name);

    const passwordRequests: Request[] = [];
    page.on('request', (request) => {
      if (requestPath(request) === PASSWORD_PATH) passwordRequests.push(request);
    });

    await page.goto('/login');
    expect(await page.evaluate(() => navigator.webdriver)).toBe(true);
    const email = uniqueEmail('automation');
    const response = await submitIdentifierAfterRateLimitReset(page, email);

    expect(response.status()).toBe(403);
    expect(await responseErrorCode(response)).toBe('bot_detected');
    expect(response.request().postDataJSON()).toMatchObject({
      email,
      decoy_link_clicked: false,
      bot_signals: { auto_webdriver: true },
    });
    expect(response.request().postDataJSON()).toMatchObject({
      pow_nonce: expect.any(String),
      pow_solution: expect.any(String),
      device_fingerprint: expect.any(Object),
    });

    await expect(page.getByText(/vous ne pouvez pas effectuer cette action/iu)).toBeVisible();
    await expect(page.getByLabel('Email', { exact: true })).toBeEditable();
    await expect(page.getByRole('button', { name: 'Continuer' })).toBeEnabled();
    await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
    expect(passwordRequests).toHaveLength(0);
    await expectNoAuthenticatedSession(page);
    await expectHardenedBlockedResponse(response);
    await expectBlockedResponseHasNoAuthenticationMaterial(response, email);
  });

  test('automation rejection is independent of account existence', async ({ page }, testInfo) => {
    requireReferenceBrowser(testInfo.project.name);
    await page.goto('/login');

    const responses = [];
    for (const email of [uniqueEmail('unknown'), 'admin@example.test']) {
      const response = await submitIdentifierAfterRateLimitReset(page, email);
      responses.push({
        code: await responseErrorCode(response),
        status: response.status(),
        text: await response.text(),
      });
    }

    expect(responses.map(({ status }) => status)).toEqual([403, 403]);
    expect(responses.map(({ code }) => code)).toEqual(['bot_detected', 'bot_detected']);
    expect(responses[0]?.text).not.toContain('unknown-');
    expect(responses[1]?.text).not.toContain('admin@example.test');
    await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
    await expectNoAuthenticatedSession(page);
  });

  for (const scenario of BOT_GUARD_ATTACKS) {
    test(`${scenario.name} is blocked before password challenge issuance`, async ({
      page,
    }, testInfo) => {
      requireReferenceBrowser(testInfo.project.name);
      await rewriteIdentifierRequest(page, scenario.mutate);
      const passwordRequests: Request[] = [];
      page.on('request', (request) => {
        if (requestPath(request) === PASSWORD_PATH) passwordRequests.push(request);
      });

      await page.goto('/login');
      const email = uniqueEmail('tampered');
      const response = await submitIdentifierAfterRateLimitReset(page, email);

      expect(response.status()).toBe(403);
      expect(await responseErrorCode(response)).toBe(scenario.expectedCode);
      expect(passwordRequests).toHaveLength(0);
      await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
      await expect(page.getByLabel('Email', { exact: true })).toBeEditable();
      await expectNoAuthenticatedSession(page);
      await expectHardenedBlockedResponse(response);
      expect(response.request().timing().responseEnd).toBeGreaterThanOrEqual(700);
      expect(response.request().timing().responseEnd).toBeLessThan(10_000);
      await expectBlockedResponseHasNoAuthenticationMaterial(response, email);
    });
  }

  for (const scenario of POW_ATTACKS) {
    test(`${scenario.name} cannot bypass proof of work`, async ({ page }, testInfo) => {
      requireReferenceBrowser(testInfo.project.name);
      await rewriteIdentifierRequest(page, scenario.mutate);
      await page.goto('/login');

      const response = await submitIdentifierAfterRateLimitReset(page, uniqueEmail('pow-tamper'));

      expect(response.status()).toBe(400);
      expect(await responseErrorCode(response)).toBe(scenario.expectedCode);
      await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
      await expectNoAuthenticatedSession(page);
      await expectHardenedBlockedResponse(response);
    });
  }

  test('a consumed proof of work cannot be replayed', async ({ page }, testInfo) => {
    requireReferenceBrowser(testInfo.project.name);
    await page.goto('/login');
    const firstResponse = await submitIdentifierAfterRateLimitReset(
      page,
      uniqueEmail('pow-first-use'),
    );
    expect(firstResponse.status()).toBe(403);
    const firstPayload = jsonObject(firstResponse.request().postDataJSON());

    let replay = await page.request.post(IDENTIFIER_PATH, {
      data: firstPayload,
      headers: { Origin: new URL(page.url()).origin },
    });

    if (replay.status() === 429) {
      expectRateLimitHeaders(replay.headers());
      await waitForRateLimitReset(page, replay.headers());
      replay = await page.request.post(IDENTIFIER_PATH, {
        data: firstPayload,
        headers: { Origin: new URL(page.url()).origin },
      });
    }

    expect(replay.status()).toBe(400);
    expect(await responseErrorCode(replay)).toBe('pow_nonce_consumed');
    expect(await replay.text()).not.toContain('state_token');
    await expectNoAuthenticatedSession(page);
  });

  test('identifier bursts are rate limited without issuing authentication material', async ({
    page,
  }, testInfo) => {
    requireReferenceBrowser(testInfo.project.name);
    await page.goto('/login');

    let limitedResponse: Response | undefined;
    let limitedEmail = '';
    for (let attempt = 0; attempt <= 10; attempt += 1) {
      const email = uniqueEmail('burst');
      const response = await submitIdentifier(page, email);
      if (response.status() === 429) {
        limitedResponse = response;
        limitedEmail = email;
        break;
      }

      expect(response.status()).toBe(403);
      expect(await responseErrorCode(response)).toBe('bot_detected');
      await page.reload();
    }

    expect(
      limitedResponse,
      'A burst of at most eleven identifier requests must be rate limited.',
    ).toBeDefined();
    if (!limitedResponse) throw new Error('Identifier rate limit was not enforced.');

    expect(await responseErrorCode(limitedResponse)).toBe('rate_limited');
    expectRateLimitHeaders(await limitedResponse.allHeaders());
    await expectHardenedBlockedResponse(limitedResponse);
    await expectBlockedResponseHasNoAuthenticationMaterial(limitedResponse, limitedEmail);
    await expect(page.getByLabel('Mot de passe', { exact: true })).not.toBeVisible();
    await expectNoAuthenticatedSession(page);
  });

  test('modern automation and reverse-engineered evasions remain fail-closed', async ({
    context,
    page,
  }, testInfo) => {
    test.setTimeout(300_000);
    requireReferenceBrowser(testInfo.project.name);

    for (const attack of CLIENT_PROBE_ATTACKS) {
      await expectClientProbeAttack(context, attack);
    }

    const forwardedIpPage = await context.newPage();
    try {
      await expectForwardedIpRotationRateLimitedSoft(forwardedIpPage);
    } finally {
      await forwardedIpPage.close();
    }

    let activeAttack = REQUEST_EVASION_ATTACKS[0];
    if (!activeAttack) throw new Error('At least one request evasion attack is required.');
    await rewriteIdentifierRequest(
      page,
      (body) => activeAttack.mutateBody(body),
      (headers) => activeAttack.mutateHeaders?.(headers) ?? headers,
    );
    await page.goto('/login');

    const decoys = page.locator('#decoy-links-container a[aria-hidden="true"][tabindex="-1"]');
    await expect
      .soft(decoys, 'The login form must mount all three anti-crawler decoy links.')
      .toHaveCount(3, { timeout: 5_000 });
    if ((await decoys.count()) === 3) {
      for (const decoy of await decoys.all()) {
        await expect(decoy).toBeHidden();
        await expect(decoy).toHaveAttribute('href', '#');
      }
    }

    for (const attack of REQUEST_EVASION_ATTACKS) {
      activeAttack = attack;
      await page.goto('/login');
      const email = uniqueEmail('request-evasion');
      const response = await submitIdentifierAfterRateLimitReset(page, email);
      await expectIdentifierBlockedSoft(response, email, attack.name);
      await expectNoAuthenticatedSession(page);
    }

    const baseURL = testInfo.project.use.baseURL;
    if (typeof baseURL !== 'string') throw new Error('The Chromium project must define baseURL.');
    const stealthBrowser = await chromium.launch({
      headless: true,
      args: ['--disable-blink-features=AutomationControlled'],
    });
    try {
      const userAgentProbe = await stealthBrowser.newPage();
      const userAgent = (await userAgentProbe.evaluate(() => navigator.userAgent)).replace(
        'HeadlessChrome/',
        'Chrome/',
      );
      await userAgentProbe.close();
      const stealthContext = await stealthBrowser.newContext({
        baseURL,
        locale: 'fr-FR',
        timezoneId: 'Europe/Paris',
        userAgent,
      });
      await stealthContext.addInitScript({ content: FULL_STEALTH_INIT_SCRIPT });
      const stealthPage = await stealthContext.newPage();
      await stealthPage.goto('/login');
      expect(await stealthPage.evaluate(() => navigator.webdriver)).toBe(false);

      const email = uniqueEmail('full-stealth');
      const response = await submitIdentifierAfterRateLimitReset(
        stealthPage,
        email,
        submitIdentifierLikeHuman,
      );
      await expectIdentifierBlockedSoft(
        response,
        email,
        'AutomationControlled disabled with coherent JS and fingerprint spoofing',
      );
      await expectNoAuthenticatedSession(stealthPage);
      await stealthContext.close();
    } finally {
      await stealthBrowser.close();
    }
  });
});
