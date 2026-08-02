import { expect, test } from '@playwright/test';

import {
  assertBrowserCookieContract,
  csrfRequestHeaders,
  loginThroughBrowser,
  requiredEnvironment,
} from './identity-journey.support';

test.use({
  launchOptions: {
    args: ['--disable-blink-features=AutomationControlled'],
  },
  trace: 'off',
  video: 'off',
});

test.describe('@staging authenticated Identity acceptance', () => {
  test('the synthetic release identity can sign in, access itself and sign out', async ({
    page,
  }) => {
    const email = requiredEnvironment('NVBES_STAGING_IDENTITY_EMAIL');
    const password = requiredEnvironment('NVBES_STAGING_IDENTITY_PASSWORD');

    await loginThroughBrowser(page, email, password);
    await assertBrowserCookieContract(page);

    const meResponse = await page.request.get('/auth/me', { failOnStatusCode: false });
    if (meResponse.status() !== 200) {
      throw new Error('Authenticated Identity pre-release check failed.');
    }
    const body: unknown = await meResponse.json();
    if (!isExpectedIdentity(body, email)) {
      throw new Error('Authenticated Identity pre-release identity does not match the fixture.');
    }

    const logoutResponse = await page.request.post('/auth/logout', {
      data: {},
      failOnStatusCode: false,
      headers: await csrfRequestHeaders(page),
    });
    expect(logoutResponse.status()).toBe(200);

    await page.goto('/login');
    await expect(page.getByRole('heading', { name: 'Se connecter' })).toBeVisible();
  });
});

function isExpectedIdentity(value: unknown, email: string): boolean {
  if (typeof value !== 'object' || value === null || !('user' in value)) {
    return false;
  }
  const user = value.user;
  return (
    typeof user === 'object' &&
    user !== null &&
    'email' in user &&
    typeof user.email === 'string' &&
    user.email.toLowerCase() === email.toLowerCase()
  );
}
