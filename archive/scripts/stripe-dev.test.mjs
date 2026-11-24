import assert from 'node:assert/strict';
import test from 'node:test';
import { localDatabase } from './stripe-dev.mjs';
import { validateSandboxConfig, verifySandbox } from './lib/stripe-sandbox.mjs';

const config = {
  NVBES_STRIPE_SECRET_KEY: 'rk_test_fixture',
  NVBES_STRIPE_ACCOUNT_ID: 'acct_fixture',
};
test('refuses live keys, missing account, and alternate credential destinations', () => {
  assert.doesNotThrow(() => validateSandboxConfig(config));
  for (const override of [
    { NVBES_STRIPE_SECRET_KEY: 'sk_live_fixture' },
    { NVBES_STRIPE_ACCOUNT_ID: '' },
    { NVBES_STRIPE_API_BASE_URL: 'https://example.com' },
    { NVBES_STRIPE_SECRET_KEY: 'sk_test_dummy' },
  ])
    assert.throws(() => validateSandboxConfig({ ...config, ...override }));
});
test('refuses remote and non-development databases before seeding', () => {
  localDatabase('postgres://localhost/nvbes_dev_billing');
  localDatabase('postgres://postgres/nvbes_billing_test_ci');
  assert.throws(() => localDatabase('postgres://localhost/nvbes_billing'));
  assert.throws(() => localDatabase('postgres://production.example/nvbes_dev_billing'));
});
test('binds requests to the expected account and rejects unauthorized contexts', async (t) => {
  t.mock.method(globalThis, 'fetch', async (_url, options) => {
    assert.equal(options.headers['Stripe-Account'], 'acct_fixture');
    return new Response('{}', { status: 403 });
  });
  await assert.rejects(verifySandbox(config), /HTTP 403/);
});
test('requires positive test-mode evidence', async (t) => {
  t.mock.method(
    globalThis,
    'fetch',
    async (url) =>
      new Response(
        JSON.stringify(url.endsWith('/account') ? { id: 'acct_fixture' } : { livemode: true }),
      ),
  );
  await assert.rejects(verifySandbox(config), /Live Stripe object/);
});
