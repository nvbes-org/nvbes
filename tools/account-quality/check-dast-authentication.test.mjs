import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { checkDastAuthentication } from './check-dast-authentication.mjs';

const subject = '11111111-1111-4111-8111-111111111111';
const tenant = '22222222-2222-4222-8222-222222222222';
const env = {
  ACCOUNT_URL: 'https://account.staging.example.test',
  API_URL: 'https://api.staging.example.test',
  BACKOFFICE_URL: 'https://backoffice.staging.example.test',
  DAST_ACCOUNT_SESSION_COOKIE: '__Host-session=account-secret',
  DAST_ALLOWED_ORIGINS:
    'https://account.staging.example.test,https://api.staging.example.test,https://backoffice.staging.example.test',
  DAST_API_AUTHORIZATION: 'Bearer api-secret',
  DAST_BACKOFFICE_SESSION_COOKIE: '__Host-session=backoffice-secret',
  DAST_EXPECTED_SUBJECT_ID: subject,
  DAST_EXPECTED_TENANT_ID: tenant,
};
const resolver = async () => [{ address: '8.8.8.8', family: 4 }];

test('proves identity binding and exact protected fixture access', async () => {
  const requests = [];
  const fetchImpl = async (url, init) => {
    requests.push({ init, url: url.toString() });
    if (url.pathname === '/auth/me') {
      return jsonResponse({ current_tenant_id: tenant, user: { id: subject } });
    }
    if (url.pathname === '/oauth/userinfo') {
      return jsonResponse({ sub: subject, tenant_id: tenant });
    }
    return jsonResponse({ principal_id: subject, tenant_id: tenant });
  };

  const evidence = await checkDastAuthentication({
    env,
    fetchImpl,
    resolver,
    verifiedAt: new Date('2026-07-29T10:00:00.000Z'),
  });

  assert.deepEqual(
    requests.map(({ url }) => new URL(url).pathname),
    ['/auth/me', '/oauth/userinfo', `/admin/users/${subject}`],
  );
  assert.equal(requests[0].init.headers.Cookie, env.DAST_ACCOUNT_SESSION_COOKIE);
  assert.equal(requests[1].init.headers.Authorization, env.DAST_API_AUTHORIZATION);
  assert.equal(requests[2].init.headers.Cookie, env.DAST_BACKOFFICE_SESSION_COOKIE);
  assert.equal(
    requests.every(({ init }) => init.redirect === 'manual'),
    true,
  );
  assert.equal(evidence.checks.length, 3);
  assert.equal(evidence.accountAndApiIdentityBoundToFixture, true);
  assert.equal(evidence.backofficeCredentialAuthorizedForFixture, true);
  assert.doesNotMatch(JSON.stringify(evidence), /account-secret|api-secret|backoffice-secret/u);
});

test('fails closed when an expired credential receives 401', async () => {
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async () => jsonResponse({}, 401),
      resolver,
    }),
    /expected HTTP 200 and received 401/u,
  );
});

test('fails closed when the credential belongs to another identity or tenant', async () => {
  const otherSubject = '33333333-3333-4333-8333-333333333333';
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async (url) => {
        if (url.pathname === '/auth/me') {
          return jsonResponse({
            current_tenant_id: tenant,
            user: { id: otherSubject },
          });
        }
        return jsonResponse({
          principal_id: subject,
          sub: subject,
          tenant_id: tenant,
        });
      },
      resolver,
    }),
    /account authenticated preflight rejected user\.id/u,
  );

  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async (url) => {
        if (url.pathname === '/auth/me') {
          return jsonResponse({
            current_tenant_id: tenant,
            user: { id: subject },
          });
        }
        if (url.pathname === '/oauth/userinfo') {
          return jsonResponse({
            sub: subject,
            tenant_id: '44444444-4444-4444-8444-444444444444',
          });
        }
        return jsonResponse({ principal_id: subject, tenant_id: tenant });
      },
      resolver,
    }),
    /api authenticated preflight rejected tenant_id/u,
  );
});

test('does not follow redirects with an authenticated request', async () => {
  const observed = [];
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async (_url, init) => {
        observed.push(init.redirect);
        return new Response(null, {
          headers: { location: 'https://account.nvbes.fr/' },
          status: 302,
        });
      },
      resolver,
    }),
    /expected HTTP 200 and received 302/u,
  );
  assert.deepEqual(observed, ['manual']);
});

test('rejects private DNS before sending credentials', async () => {
  let requested = false;
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async () => {
        requested = true;
        return jsonResponse({});
      },
      resolver: async () => [{ address: '127.0.0.1', family: 4 }],
    }),
    /DNS includes a non-public address/u,
  );
  assert.equal(requested, false);
});

test('suppresses response bodies and transport errors from failure messages', async () => {
  const sensitiveBody = 'private-response-payload';
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async () =>
        new Response(sensitiveBody, {
          headers: { 'content-type': 'application/json' },
          status: 200,
        }),
      resolver,
    }),
    (error) => {
      assert.doesNotMatch(error.message, new RegExp(sensitiveBody, 'u'));
      return /invalid JSON/u.test(error.message);
    },
  );

  const sensitiveTransportError = 'Bearer transport-secret';
  await assert.rejects(
    checkDastAuthentication({
      env,
      fetchImpl: async () => {
        throw new Error(sensitiveTransportError);
      },
      resolver,
    }),
    (error) => {
      assert.doesNotMatch(error.message, new RegExp(sensitiveTransportError, 'u'));
      return /authenticated preflight request failed/u.test(error.message);
    },
  );
});

test('workflow brackets every ZAP scan with authenticated proofs', () => {
  const workflow = readFileSync('.github/workflows/dast.yml', 'utf8');
  for (const [surface, scan] of [
    ['account', 'zap-baseline.py -t "$ACCOUNT_URL"'],
    ['api', 'zap-api-scan.py -t "$OPENAPI_URL"'],
    ['backoffice', 'zap-baseline.py -t "$BACKOFFICE_URL"'],
  ]) {
    const before = workflow.indexOf(`auth-preflight-${surface}-before.json`);
    const scanPosition = workflow.indexOf(scan);
    const after = workflow.indexOf(`auth-preflight-${surface}-after.json`);
    assert.ok(before >= 0 && before < scanPosition, `${surface} before proof`);
    assert.ok(scanPosition >= 0 && scanPosition < after, `${surface} after proof`);
  }
  assert.equal(
    workflow.match(/node tools\/account-quality\/check-dast-authentication\.mjs/gu)?.length,
    6,
  );
  assert.equal(workflow.match(/scan_status=\$\?/gu)?.length, 3);
  assert.match(workflow, /DAST_EXPECTED_SUBJECT_ID:.*vars\.DAST_EXPECTED_SUBJECT_ID/u);
  assert.match(workflow, /DAST_EXPECTED_TENANT_ID:.*vars\.DAST_EXPECTED_TENANT_ID/u);
  assert.match(workflow, /DAST_ACCOUNT_SESSION_COOKIE:.*secrets\.DAST_ACCOUNT_SESSION_COOKIE/u);
  assert.match(workflow, /DAST_API_AUTHORIZATION:.*secrets\.DAST_API_AUTHORIZATION/u);
  assert.match(
    workflow,
    /DAST_BACKOFFICE_SESSION_COOKIE:.*secrets\.DAST_BACKOFFICE_SESSION_COOKIE/u,
  );
  assert.doesNotMatch(
    workflow,
    /check-dast-authentication\.mjs[^\n]+(?:DAST_|AUTHORIZATION|COOKIE)/u,
  );
  assert.match(workflow, /-j --client-spider/u);
  assert.match(workflow, /environment:\s*\n\s+name: account-dast-staging/u);
  assert.match(workflow, /\[\[ "\$GITHUB_REF" == "refs\/heads\/main" \]\]/u);
  assert.match(workflow, /ref: \$\{\{ github\.sha \}\}/u);
});

function jsonResponse(body, status = 200) {
  return new Response(JSON.stringify(body), {
    headers: { 'content-type': 'application/json' },
    status,
  });
}
