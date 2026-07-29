import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  isPublicAddress,
  parseOriginAllowlist,
  productionOrigins,
  validateDastTarget,
  verifyRedirectChain,
} from './validate-dast-targets.mjs';

const allowed = parseOriginAllowlist(
  'https://account.staging.example.test,https://api.staging.example.test',
);
const forbidden = productionOrigins();

test('accepts only exact HTTPS staging origins', () => {
  assert.equal(
    validateDastTarget('https://api.staging.example.test/openapi.json', allowed, forbidden)
      .pathname,
    '/openapi.json',
  );
  assert.throws(
    () => validateDastTarget('https://attacker.test/', allowed, forbidden),
    /exact staging allowlist/u,
  );
  assert.throws(
    () => validateDastTarget('http://api.staging.example.test/', allowed, forbidden),
    /must use HTTPS/u,
  );
});

test('rejects production, userinfo and private literals', () => {
  assert.throws(
    () => validateDastTarget('https://account.nvbes.fr/', allowed, forbidden),
    /Production DAST target/u,
  );
  assert.throws(
    () => validateDastTarget('https://api.staging.example.test@127.0.0.1/', allowed, forbidden),
    /URL credentials/u,
  );
  const privateAllowed = new Set(['https://127.0.0.1']);
  assert.throws(
    () => validateDastTarget('https://127.0.0.1/', privateAllowed, forbidden),
    /non-public host/u,
  );
});

test('rejects private, loopback and reserved DNS ranges', () => {
  for (const address of [
    '0.0.0.0',
    '10.0.0.1',
    '100.64.0.1',
    '127.0.0.1',
    '169.254.1.1',
    '172.16.0.1',
    '192.168.1.1',
    '192.0.2.10',
    '198.18.0.1',
    '198.51.100.10',
    '203.0.113.10',
    '::',
    '::1',
    '2001:db8::1',
    'fc00::1',
    'fe80::1',
  ]) {
    assert.equal(isPublicAddress(address), false, address);
  }
  assert.equal(isPublicAddress('8.8.8.8'), true);
  assert.equal(isPublicAddress('2606:4700:4700::1111'), true);
});

test('validates every redirect before following it', async () => {
  const fetchImpl = async () =>
    new Response(null, {
      headers: { location: 'https://account.nvbes.fr/login' },
      status: 302,
    });
  const resolver = async () => [{ address: '8.8.8.8', family: 4 }];

  await assert.rejects(
    verifyRedirectChain(new URL('https://account.staging.example.test/'), allowed, forbidden, {
      fetchImpl,
      resolver,
    }),
    /Production DAST target/u,
  );
});

test('forces the API scan onto the separately validated staging host', () => {
  const workflow = readFileSync('.github/workflows/dast.yml', 'utf8');
  assert.match(workflow, /API_URL:.*DAST_API_URL/u);
  assert.match(workflow, /new URL\(process\.env\.API_URL\)\.host/u);
  assert.match(workflow, /zap-api-scan\.py[\s\S]*-O "\$API_OVERRIDE"/u);
  assert.match(workflow, /ZAP_AUTH_HEADER_SITE=.*process\.env\.API_URL/u);
  assert.doesNotMatch(workflow, /ZAP_AUTH_HEADER_SITE=.*process\.env\.OPENAPI_URL/u);
});
