import assert from 'node:assert/strict';
import test from 'node:test';

import { checkProductionAccountTarget } from './check-production-account-target.mjs';

const RELEASE = 'a'.repeat(40);
const input = {
  allowedOrigins: 'https://account.example.test,https://api.example.test',
  apiBaseUrl: 'https://api.example.test',
  expectedRelease: RELEASE,
  resolver: async () => [{ address: '8.8.8.8', family: 4 }],
  webBaseUrl: 'https://account.example.test',
};

test('proves exact public production origins and deployed release', async () => {
  const requests = [];
  const result = await checkProductionAccountTarget({
    ...input,
    fetchImpl: async (url, init) => {
      requests.push({ init, url: url.toString() });
      return url.pathname === '/health'
        ? jsonResponse({ release_id: RELEASE, status: 'ok' })
        : new Response('account', { status: 200 });
    },
  });
  assert.deepEqual(result, {
    apiOrigin: 'https://api.example.test',
    release: RELEASE,
    webOrigin: 'https://account.example.test',
  });
  assert.deepEqual(
    requests.map(({ url }) => url),
    ['https://account.example.test/', 'https://api.example.test/health'],
  );
  assert.equal(
    requests.every(({ init }) => init.redirect === 'manual'),
    true,
  );
});

test('rejects local, private, reserved and non-allowlisted targets', async () => {
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      apiBaseUrl: 'https://localhost',
      allowedOrigins: 'https://account.example.test,https://localhost',
    }),
    /local hostname/u,
  );
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      resolver: async () => [{ address: '203.0.113.10', family: 4 }],
    }),
    /non-public or reserved/u,
  );
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      webBaseUrl: 'https://attacker.example.test',
    }),
    /exact production allowlist/u,
  );
});

test('rejects redirects and a target reporting another release', async () => {
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      fetchImpl: async () =>
        new Response(null, {
          headers: { location: 'https://attacker.example.test' },
          status: 302,
        }),
    }),
    /expected 2xx and received 302/u,
  );
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      fetchImpl: async (url) =>
        url.pathname === '/health'
          ? jsonResponse({ release_id: 'b'.repeat(40), status: 'ok' })
          : new Response('account', { status: 200 }),
    }),
    /release does not match/u,
  );
});

test('rejects malformed or oversized health evidence', async () => {
  await assert.rejects(
    checkProductionAccountTarget({
      ...input,
      fetchImpl: async (url) =>
        url.pathname === '/health'
          ? new Response('not-json', {
              headers: { 'content-type': 'text/plain' },
              status: 200,
            })
          : new Response('account', { status: 200 }),
    }),
    /must return JSON/u,
  );
});

function jsonResponse(body) {
  return new Response(JSON.stringify(body), {
    headers: { 'content-type': 'application/json' },
    status: 200,
  });
}
