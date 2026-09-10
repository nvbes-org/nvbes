import assert from 'node:assert/strict';

export async function verifyIdentityCors(origins) {
  for (const path of ['/oauth/par', '/oauth/token', '/oauth/userinfo']) {
    for (const origin of [origins.account, 'https://unregistered.example', 'null']) {
      const response = await fetch(`${origins.identity}${path}`, {
        method: 'OPTIONS',
        signal: AbortSignal.timeout(5000),
        headers: {
          origin,
          'access-control-request-method': 'POST',
          'access-control-request-headers': 'dpop,content-type,authorization',
        },
      });
      assert.ok(response.ok, 'preflight must complete without protocol authentication');
      assert.equal(
        response.headers.get('access-control-allow-origin'),
        origin === origins.account ? origin : null,
      );
      assert.equal(response.headers.get('access-control-allow-credentials'), null);
      if (origin === origins.account) {
        const allowedHeaders = response.headers.get('access-control-allow-headers');
        for (const header of ['dpop', 'content-type', 'authorization']) {
          assert.ok(
            allowedHeaders
              ?.split(',')
              .map((value) => value.trim())
              .includes(header),
          );
        }
        assert.ok(response.headers.get('access-control-allow-methods')?.includes('POST'));
      }
      await response.arrayBuffer();
    }
    const response = await fetch(`${origins.identity}${path}`, {
      method: 'POST',
      headers: { origin: origins.account, 'content-type': 'application/x-www-form-urlencoded' },
      body: '',
      signal: AbortSignal.timeout(5000),
    });
    assert.ok(response.status >= 400, 'CORS must not bypass protocol authentication');
    assert.equal(response.headers.get('access-control-allow-origin'), origins.account);
    assert.ok(response.headers.get('access-control-expose-headers')?.includes('dpop-nonce'));
    await response.arrayBuffer();
  }
  for (const path of ['/oauth/jwks', '/.well-known/openid-configuration']) {
    const response = await fetch(`${origins.identity}${path}`, {
      headers: { origin: origins.account },
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('access-control-allow-origin'), '*');
    assert.equal(response.headers.get('access-control-allow-credentials'), null);
    await response.arrayBuffer();
  }
  for (const path of ['/oauth/logout', '/oauth/introspect', '/oauth/authorize/login']) {
    const response = await fetch(`${origins.identity}${path}`, {
      method: 'OPTIONS',
      signal: AbortSignal.timeout(5000),
      headers: { origin: origins.account, 'access-control-request-method': 'POST' },
    });
    assert.equal(response.headers.get('access-control-allow-origin'), null);
    await response.arrayBuffer();
  }
}
