import assert from 'node:assert/strict';

export async function verifyResourceCors(origins, endpoints) {
  for (const endpoint of Object.values(endpoints)) {
    for (const origin of [origins.account, `${origins.account}.attacker.test`, 'null']) {
      const response = await fetch(endpoint, {
        method: 'OPTIONS',
        signal: AbortSignal.timeout(5000),
        headers: {
          origin,
          'access-control-request-method': 'GET',
          'access-control-request-headers': 'authorization,dpop,content-type,idempotency-key',
        },
      });
      assert.ok(response.ok);
      assert.equal(
        response.headers.get('access-control-allow-origin'),
        origin === origins.account ? origin : null,
      );
      assert.equal(response.headers.get('access-control-allow-credentials'), null);
      const headers = response.headers
        .get('access-control-allow-headers')
        ?.split(',')
        .map((value) => value.trim());
      for (const header of ['authorization', 'dpop', 'content-type', 'idempotency-key']) {
        assert.ok(headers?.includes(header));
      }
      assert.ok(response.headers.get('vary')?.includes('origin'));
      await response.arrayBuffer();
    }
    const response = await fetch(endpoint, {
      headers: { origin: origins.account },
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 401, 'CORS must not authenticate an API request');
    assert.equal(response.headers.get('access-control-allow-origin'), origins.account);
    assert.ok(response.headers.get('access-control-expose-headers')?.includes('dpop-nonce'));
    await response.arrayBuffer();
  }
  for (const [service, paths] of Object.entries({
    account: ['/internal/v1/billing/authorize', '/metrics', '/health/ready'],
    billing: ['/webhooks/stripe', '/operator/billing/overview', '/metrics', '/health/ready'],
  })) {
    for (const path of paths) {
      const response = await fetch(`${origins[service]}${path}`, {
        method: 'OPTIONS',
        headers: { origin: origins.account, 'access-control-request-method': 'POST' },
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(response.headers.get('access-control-allow-origin'), null, `${service}${path}`);
      await response.arrayBuffer();
    }
  }
}
