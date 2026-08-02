import { expect, type Locator, type Page, type TestInfo } from '@playwright/test';

export type Random = () => number;

export type JournalEntry = {
  action: string;
  detail: string;
  iteration: number;
  urlAfter: string;
  urlBefore: string;
};

export type Diagnostics = {
  consoleErrors: string[];
  failedRequests: string[];
  pageErrors: string[];
  serverErrors: string[];
};

export async function randomLocator(locator: Locator, random: Random): Promise<Locator | null> {
  const count = await locator.count();
  return count === 0 ? null : locator.nth(Math.floor(random() * count));
}

export async function expectHealthyDocument(
  page: Page,
  seed: number,
  entry: JournalEntry | undefined,
  diagnostics: Diagnostics,
): Promise<void> {
  const context = `seed=${seed} iteration=${entry?.iteration ?? -1} action=${entry?.action ?? 'unknown'} url=${page.url()}`;
  await expect(page.locator('#root'), context).toHaveCount(1);
  expect(new URL(page.url()).origin, context).toBe(
    new URL(process.env.NVBES_WEB_BASE_URL ?? 'http://127.0.0.1:3001').origin,
  );

  try {
    await expect(page.locator('#root > *').first(), context).toBeVisible();
  } catch (error) {
    if (diagnostics.pageErrors.length > 0) {
      throw new Error(`${context}; page errors: ${diagnostics.pageErrors.join(' | ')}`, {
        cause: error,
      });
    }
    throw error;
  }

  const documentState = await page.evaluate(() => ({
    bodyChildren: document.body.childElementCount,
    clientWidth: document.documentElement.clientWidth,
    rootChildren: document.querySelector('#root')?.childElementCount ?? 0,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  expect(documentState.bodyChildren, context).toBeGreaterThan(0);
  expect(documentState.rootChildren, context).toBeGreaterThan(0);
  expect(documentState.scrollWidth, context).toBeLessThanOrEqual(documentState.clientWidth + 2);
}

export function collectDiagnostics(page: Page, webOrigin: string): Diagnostics {
  const applicationOrigins = configuredApplicationOrigins(webOrigin);
  const isApplicationUrl = (value: string): boolean => {
    try {
      return applicationOrigins.has(new URL(value).origin);
    } catch {
      return false;
    }
  };
  const diagnostics: Diagnostics = {
    consoleErrors: [],
    failedRequests: [],
    pageErrors: [],
    serverErrors: [],
  };

  page.on('pageerror', (error) => diagnostics.pageErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) {
      diagnostics.consoleErrors.push(message.text());
    }
  });
  page.on('response', (response) => {
    if (response.status() >= 500 && isApplicationUrl(response.url())) {
      diagnostics.serverErrors.push(`${response.status()} ${response.url()}`);
    }
  });
  page.on('requestfailed', (request) => {
    const failure = request.failure()?.errorText ?? 'unknown failure';
    if (isApplicationUrl(request.url()) && !failure.includes('ERR_ABORTED')) {
      diagnostics.failedRequests.push(`${failure} ${request.method()} ${request.url()}`);
    }
  });
  return diagnostics;
}

export async function attachMonkeyRun(
  testInfo: TestInfo,
  seed: number,
  actionCount: number,
  journal: JournalEntry[],
  diagnostics: Diagnostics,
): Promise<void> {
  await testInfo.attach('monkey-run', {
    body: JSON.stringify({ actionCount, diagnostics, journal, seed }, null, 2),
    contentType: 'application/json',
  });
}

function configuredApplicationOrigins(webOrigin: string): ReadonlySet<string> {
  const origins = new Set([webOrigin]);
  for (const value of [
    process.env.NVBES_IDENTITY_SERVICE_BASE_URL,
    process.env.VITE_IDENTITY_SERVICE_BASE_URL,
  ]) {
    if (value) origins.add(new URL(value, webOrigin).origin);
  }
  return origins;
}

export function diagnosticMessage(seed: number, category: string): string {
  return `${category}; replay with NVBES_MONKEY_SEED=${seed}`;
}

export function boundedInteger(
  value: string | undefined,
  fallback: number,
  minimum: number,
  maximum: number,
): number {
  const parsed = value === undefined ? fallback : Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`Expected an integer between ${minimum} and ${maximum}, received ${value}`);
  }
  return parsed;
}

export function pick<T>(values: readonly T[], random: Random): T {
  const value = values[Math.floor(random() * values.length)];
  if (value === undefined) throw new Error('Cannot select from an empty collection');
  return value;
}

export function randomToken(random: Random): string {
  return Math.floor(random() * 0xffff_ffff)
    .toString(36)
    .padStart(7, '0');
}

export function xorshift(seed: number): Random {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 4_294_967_296;
  };
}
