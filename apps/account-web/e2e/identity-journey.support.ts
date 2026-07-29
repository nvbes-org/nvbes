import { expect, type Page } from '@playwright/test';

export async function loginThroughBrowser(page: Page, email: string, password: string) {
  await page.goto('/login');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Continuer' }).click();
  await expect(page.getByLabel('Mot de passe', { exact: true })).toBeVisible();
  await page.getByLabel('Mot de passe', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Se connecter' }).click();
  await expect(page).toHaveURL(/\/account\/0\/?$/u);
}

export function emailTextbox(page: Page) {
  return page.getByLabel('Email', { exact: true });
}

export async function assertBrowserCookieContract(page: Page) {
  const currentUrl = new URL(page.url());
  const secureContext = currentUrl.protocol === 'https:';
  const expectedSessionName = secureContext ? '__Host-session' : 'session';
  const expectedCsrfName = secureContext ? '__Host-csrf_token' : 'csrf_token';
  const cookies = await page.context().cookies();
  const sessionCookie = cookies.find((cookie) => cookie.name === expectedSessionName);
  const csrfCookie = cookies.find((cookie) => cookie.name === expectedCsrfName);

  expect(sessionCookie, `authenticated cookie ${expectedSessionName}`).toBeDefined();
  expect(sessionCookie?.httpOnly).toBe(true);
  expect(sessionCookie?.sameSite).toBe('Strict');
  expect(sessionCookie?.path).toBe('/');
  expect(sessionCookie?.domain).toBe(currentUrl.hostname);
  expect(sessionCookie?.secure).toBe(secureContext);
  expect(sessionCookie?.expires ?? 0).toBeGreaterThan(Date.now() / 1000);

  expect(csrfCookie, `double-submit cookie ${expectedCsrfName}`).toBeDefined();
  expect(csrfCookie?.httpOnly).toBe(false);
  expect(csrfCookie?.sameSite).toBe('Strict');
  expect(csrfCookie?.path).toBe('/');
  expect(csrfCookie?.domain).toBe(currentUrl.hostname);
  expect(csrfCookie?.secure).toBe(secureContext);
  expect(csrfCookie?.expires ?? 0).toBeGreaterThan(Date.now() / 1000);
}

export async function csrfRequestHeaders(page: Page) {
  const cookies = await page.context().cookies();
  const csrfCookie = cookies.find((cookie) => cookie.name.endsWith('csrf_token'));

  if (!csrfCookie) {
    throw new Error('CSRF cookie is required for authenticated mutating requests.');
  }

  return {
    Origin: new URL(page.url()).origin,
    'X-CSRF-Token': csrfCookie.value,
  };
}

export function requiredEnvironment(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required for account browser tests.`);
  }
  return value;
}
