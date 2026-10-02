import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, randomUUID, sign } from 'node:crypto';

export function dpopKey() {
  const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' });
  const jwk = publicKey.export({ format: 'jwk' });
  const jkt = createHash('sha256')
    .update(JSON.stringify({ crv: jwk.crv, kty: jwk.kty, x: jwk.x, y: jwk.y }))
    .digest('base64url');
  return {
    jkt,
    proof(method, endpoint, token, changes = {}, headerChanges = {}) {
      const url = new URL(endpoint);
      url.search = '';
      url.hash = '';
      const header = { typ: 'dpop+jwt', alg: 'ES256', jwk, ...headerChanges };
      const claims = {
        jti: randomUUID(),
        htm: method,
        htu: url.href,
        iat: Math.floor(Date.now() / 1000),
        ...changes,
      };
      if (token) claims.ath = createHash('sha256').update(token).digest('base64url');
      const input = [header, claims]
        .map((value) => Buffer.from(JSON.stringify(value)).toString('base64url'))
        .join('.');
      const signature = sign('sha256', Buffer.from(input), {
        key: privateKey,
        dsaEncoding: 'ieee-p1363',
      }).toString('base64url');
      return `${input}.${signature}`;
    },
  };
}

export async function verifyResourceDpop({ fixture, endpoints, issue, restart }) {
  const tokens = [];
  for (const service of ['account', 'billing']) {
    const key = dpopKey();
    const endpoint = endpoints[service];
    const token = await issue(service, false, `${service}:read`, key);
    tokens.push({ service, token, key });
    const request = async (proof, { scheme = 'DPoP', headers = {} } = {}) => {
      const response = await fetch(endpoint, {
        headers: {
          authorization: `${scheme} ${token}`,
          ...(proof ? { dpop: proof } : {}),
          ...headers,
        },
        signal: AbortSignal.timeout(5000),
      });
      const status = response.status;
      await response.arrayBuffer();
      return status;
    };
    assert.equal(await request(undefined), 401);
    assert.equal(await request(undefined, { scheme: 'Bearer' }), 401);
    const valid = key.proof('GET', endpoint, token);
    assert.equal(await request(valid, { scheme: 'Bearer' }), 401);
    for (const invalid of [
      key.proof('POST', endpoint, token),
      key.proof('GET', `${endpoint}/other`, token),
      key.proof('GET', endpoint, 'other-token'),
      dpopKey().proof('GET', endpoint, token),
      key.proof('GET', endpoint, token, { iat: Math.floor(Date.now() / 1000) - 1000 }),
      key.proof('GET', endpoint, token, { iat: Math.floor(Date.now() / 1000) + 1000 }),
      `${valid}, ${valid}`,
    ])
      assert.equal(await request(invalid), 401, `${service} invalid proof`);
    assert.equal(
      await request(valid, {
        headers: { 'x-forwarded-host': 'evil.invalid', 'x-forwarded-proto': 'http' },
      }),
      200,
    );
    assert.equal(await request(valid), 401);
    const concurrent = key.proof('GET', endpoint, token);
    const statuses = await Promise.all([request(concurrent), request(concurrent)]);
    assert.deepEqual(
      statuses.sort((a, b) => a - b),
      [200, 401],
    );
    await restart(service);
    assert.equal(await request(valid), 401, `${service} restart must preserve replay rejection`);
    assert.equal(await request(key.proof('GET', endpoint, token)), 200);

    // A store failure must not enter the business handler or consume the proof.
    const retry = key.proof('GET', endpoint, token);
    await fixture.sql(
      service,
      'ALTER TABLE resource_dpop_replays RENAME TO resource_dpop_replays_unavailable',
    );
    try {
      assert.equal(await request(retry), 503);
    } finally {
      await fixture.sql(
        service,
        'ALTER TABLE resource_dpop_replays_unavailable RENAME TO resource_dpop_replays',
      );
    }
    assert.equal(await request(retry), 200);

    // Fill the remaining slots with synthetic entries; never exceed the hard bound.
    await fixture.sql(
      service,
      `INSERT INTO resource_dpop_replays(bucket,jkt_hash,jti_hash,expires_at)
      SELECT b,decode(repeat('ab',32),'hex'),decode(md5(i::text)||md5(i::text),'hex'),clock_timestamp()+interval '10 minutes'
      FROM generate_series(0,63) b CROSS JOIN LATERAL generate_series(1,256-(SELECT count(*)::int FROM resource_dpop_replays WHERE bucket=b)) i`,
    );
    assert.equal(await fixture.sql(service, 'SELECT count(*) FROM resource_dpop_replays'), '16384');
    const afterCleanup = key.proof('GET', endpoint, token);
    assert.equal(await request(afterCleanup), 503);
    await fixture.sql(
      service,
      "UPDATE resource_dpop_replays SET expires_at=clock_timestamp()-interval '1 second' WHERE jkt_hash=decode(repeat('ab',32),'hex')",
    );
    assert.equal(await request(afterCleanup), 200, 'expired slots are reclaimed on demand');
    await fixture.sql(
      service,
      "DELETE FROM resource_dpop_replays WHERE jkt_hash=decode(repeat('ab',32),'hex')",
    );
    assert.equal(await request(valid), 401, 'cleanup preserves unexpired real proof records');
  }
  return tokens;
}
