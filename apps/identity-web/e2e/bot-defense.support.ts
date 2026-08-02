import {
  type BrowserContext,
  expect,
  type Page,
  type Request,
  type Response,
  type Route,
  test,
} from '@playwright/test';

export const IDENTIFIER_PATH = '/auth/challenge/identifier';
export const PASSWORD_PATH = '/auth/challenge/pwd';

export type JsonObject = Record<string, unknown>;
export type HeaderMap = Record<string, string>;

export interface ClientProbeExpectation {
  name: string;
  initScript: string;
  expectedSignals: JsonObject;
}

export function uniqueEmail(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}@example.test`;
}

export function requireReferenceBrowser(projectName: string): void {
  test.skip(projectName !== 'chromium', 'Chromium is the anti-automation reference.');
}

export async function expectClientProbeAttack(
  context: BrowserContext,
  attack: ClientProbeExpectation,
): Promise<void> {
  const attackPage = await context.newPage();
  let payload: JsonObject | undefined;
  try {
    await attackPage.addInitScript({ content: attack.initScript });
    await attackPage.route(`**${IDENTIFIER_PATH}`, async (route) => {
      payload = requestJsonObject(route);
      await route.fulfill({
        status: 403,
        json: { error: { code: 'bot_detected', message: 'Automated request detected.' } },
      });
    });
    await attackPage.goto('/login');
    const response = await submitIdentifier(attackPage, uniqueEmail('client-probe'));
    expect(response.status()).toBe(403);
    if (!payload) throw new Error(`${attack.name}: identifier payload was not captured.`);

    expect
      .soft(jsonObject(payload.bot_signals), `${attack.name}: expected detector signal.`)
      .toMatchObject(attack.expectedSignals);
  } finally {
    await attackPage.close();
  }
}

export async function expectForwardedIpRotationRateLimitedSoft(page: Page): Promise<void> {
  let spoofedAddress = 1;
  await page.route(`**${IDENTIFIER_PATH}`, async (route) => {
    const address = `203.0.113.${spoofedAddress++}`;
    await route.continue({
      headers: {
        ...route.request().headers(),
        'x-forwarded-for': address,
        'x-real-ip': address,
      },
    });
  });
  await page.goto('/login');

  let limitedResponse: Response | undefined;
  let limitedEmail = '';
  for (let attempt = 0; attempt <= 10; attempt += 1) {
    const email = uniqueEmail('forwarded-ip-rotation');
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

  expect
    .soft(
      limitedResponse,
      'Forged X-Forwarded-For and X-Real-IP values must not rotate the rate-limit identity.',
    )
    .toBeDefined();
  if (limitedResponse) {
    expect(await responseErrorCode(limitedResponse)).toBe('rate_limited');
    expectRateLimitHeaders(await limitedResponse.allHeaders());
    await expectBlockedResponseHasNoAuthenticationMaterial(limitedResponse, limitedEmail);
  }
  await expectNoAuthenticatedSession(page);
}

export async function submitIdentifier(page: Page, email: string): Promise<Response> {
  const emailInput = page.getByLabel('Email', { exact: true });
  const submitButton = page.getByRole('button', { name: 'Continuer' });
  await expect(emailInput).toBeEditable({ timeout: 30_000 });
  await expect(submitButton).toBeEnabled({ timeout: 30_000 });
  await emailInput.fill(email);
  const responsePromise = identifierResponse(page);
  await submitButton.click();
  return responsePromise;
}

export async function submitIdentifierLikeHuman(page: Page, email: string): Promise<Response> {
  const emailInput = page.getByLabel('Email', { exact: true });
  const submitButton = page.getByRole('button', { name: 'Continuer' });
  await expect(emailInput).toBeEditable({ timeout: 30_000 });
  await expect(submitButton).toBeEnabled({ timeout: 30_000 });

  const inputBox = await emailInput.boundingBox();
  const buttonBox = await submitButton.boundingBox();
  if (!inputBox || !buttonBox) throw new Error('The login controls must have a visible box.');

  await page.mouse.move(inputBox.x - 40, inputBox.y - 20);
  await page.mouse.move(inputBox.x + 20, inputBox.y + inputBox.height / 2, { steps: 12 });
  await emailInput.click();
  await page.keyboard.type(email, { delay: 73 });
  await page.mouse.move(buttonBox.x + buttonBox.width / 2, buttonBox.y + buttonBox.height / 2, {
    steps: 18,
  });
  const responsePromise = identifierResponse(page);
  await submitButton.click();
  return responsePromise;
}

export async function submitIdentifierAfterRateLimitReset(
  page: Page,
  email: string,
  submit: (page: Page, email: string) => Promise<Response> = submitIdentifier,
): Promise<Response> {
  const response = await submit(page, email);
  if (response.status() !== 429) return response;

  expect(await responseErrorCode(response)).toBe('rate_limited');
  const headers = await response.allHeaders();
  expectRateLimitHeaders(headers);
  await expectHardenedBlockedResponse(response);
  await expectBlockedResponseHasNoAuthenticationMaterial(response, email);
  await waitForRateLimitReset(page, headers);
  await page.reload();

  const retry = await submit(page, email);
  expect(retry.status(), 'Identifier request remained rate limited after Retry-After.').not.toBe(
    429,
  );
  return retry;
}

export async function rewriteIdentifierRequest(
  page: Page,
  mutateBody: (body: JsonObject) => JsonObject | Promise<JsonObject>,
  mutateHeaders?: (headers: HeaderMap) => HeaderMap,
): Promise<void> {
  await page.route(`**${IDENTIFIER_PATH}`, async (route) => {
    const body = requestJsonObject(route);
    const headers = mutateHeaders?.(route.request().headers());
    await route.continue({
      headers,
      postData: JSON.stringify(await mutateBody(body)),
    });
  });
}

export function requestJsonObject(route: Route): JsonObject {
  return jsonObject(route.request().postDataJSON());
}

export function jsonObject(body: unknown): JsonObject {
  if (typeof body !== 'object' || body === null || Array.isArray(body)) {
    throw new Error('Expected a JSON object.');
  }
  return body as JsonObject;
}

export function requestPath(request: Request): string {
  return new URL(request.url()).pathname;
}

export async function responseErrorCode(response: {
  json(): Promise<unknown>;
}): Promise<string | undefined> {
  const body: unknown = await response.json();
  if (typeof body !== 'object' || body === null || !('error' in body)) return undefined;
  const error = body.error;
  if (typeof error !== 'object' || error === null || !('code' in error)) return undefined;
  return typeof error.code === 'string' ? error.code : undefined;
}

export async function expectHardenedBlockedResponse(response: Response): Promise<void> {
  const headers = await response.allHeaders();
  expect(headers['content-type']).toContain('application/json');
  expect(headers['x-request-id']).toMatch(/^req_[a-f0-9]{32}$/u);
  expect(headers['x-content-type-options']).toBe('nosniff');
  expect(headers['set-cookie']).toBeUndefined();
}

export function expectRateLimitHeaders(headers: HeaderMap): void {
  expect(headers['retry-after']).toMatch(/^\d+$/u);
  expect(headers['ratelimit-limit']).toBe('10');
  expect(headers['ratelimit-remaining']).toBe('0');
  expect(headers['ratelimit-reset']).toMatch(/^\d+$/u);
  expect(Number(headers['retry-after'])).toBeGreaterThanOrEqual(1);
  expect(Number(headers['retry-after'])).toBeLessThanOrEqual(60);
  expect(headers['ratelimit-reset']).toBe(headers['retry-after']);
}

export async function waitForRateLimitReset(page: Page, headers: HeaderMap): Promise<void> {
  const retryAfterSeconds = Number(headers['retry-after']);
  if (!Number.isInteger(retryAfterSeconds) || retryAfterSeconds < 1 || retryAfterSeconds > 60) {
    throw new Error('Retry-After must contain a delay between 1 and 60 seconds.');
  }
  await page.waitForTimeout(retryAfterSeconds * 1_000 + 250);
}

export async function expectNoAuthenticatedSession(page: Page): Promise<void> {
  const cookies = await page.context().cookies();
  expect(cookies.map(({ name }) => name)).not.toContain('session');
  expect(cookies.map(({ name }) => name)).not.toContain('__Host-session');
}

export async function expectBlockedResponseHasNoAuthenticationMaterial(
  response: Response,
  email: string,
): Promise<void> {
  const body = await response.text();
  expect(body).not.toContain(email);
  expect(body).not.toContain('state_token');
  expect(body).not.toContain('session_token');
  expect(body).not.toContain('challenge_id');
}

export async function expectIdentifierBlockedSoft(
  response: Response,
  email: string,
  attackName: string,
): Promise<void> {
  const body = await response.text();
  expect.soft(response.status(), `${attackName}: identifier request must be rejected.`).toBe(403);
  expect
    .soft(await responseErrorCode(response), `${attackName}: stable rejection code.`)
    .toBe('bot_detected');
  expect
    .soft(body, `${attackName}: no identifier state may be issued.`)
    .not.toContain('state_token');
  expect.soft(body, `${attackName}: no session may be issued.`).not.toContain('session_token');
  expect.soft(body, `${attackName}: the identifier must not be reflected.`).not.toContain(email);
}

export function requiredString(value: unknown, name: string): string {
  if (typeof value !== 'string' || value.length === 0) throw new Error(`${name} is required.`);
  return value;
}

export async function invalidPowSolution(nonce: string): Promise<string> {
  for (let candidate = 0; ; candidate += 1) {
    const solution = `invalid-${candidate}`;
    const digest = new Uint8Array(
      await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${nonce}:${solution}`)),
    );
    if ((digest[0] ?? 0) >= 128) return solution;
  }
}

function identifierResponse(page: Page): Promise<Response> {
  return page.waitForResponse((response) => requestPath(response.request()) === IDENTIFIER_PATH, {
    timeout: 30_000,
  });
}
