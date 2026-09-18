import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';

/** All keys and processes belong to the isolated runtime fixture. */
export async function verifySigningRotation({
  origins,
  environments,
  restart,
  issue,
  access,
  before,
}) {
  const oldKid = environments.identity.NVBES_IDENTITY_TOKEN_KEY_ID;
  const oldPublic = environments.identity.NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM;
  const oldPrivate = environments.identity.NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM;
  const nextKid = 'runtime-rotated-key';
  const compareIds = (left, right) => left.localeCompare(right);
  const next = generateKeyPairSync('rsa', {
    modulusLength: 2048,
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    publicKeyEncoding: { type: 'spki', format: 'pem' },
  });
  const overlap = (kid, public_key_pem, accept_until) =>
    JSON.stringify([{ kid, public_key_pem, accept_until }]);
  const ids = async () => {
    const response = await fetch(`${origins.identity}/oauth/jwks`, {
      cache: 'no-store',
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 200);
    return (await response.json()).keys.map((key) => key.kid).sort(compareIds);
  };
  const checkPair = async (tokens, status) => {
    for (const service of ['account', 'billing']) await access(service, tokens[service], status);
  };
  // Prepublish to Identity and pin the future public key in both resource services.
  for (const service of ['identity', 'account', 'billing']) {
    environments[service].NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS = overlap(
      nextKid,
      next.publicKey,
      Math.floor(Date.now() / 1000) + 3600,
    );
    await restart(service);
  }
  assert.deepEqual(await ids(), [oldKid, nextKid].sort(compareIds));
  await checkPair(before, 200);

  Object.assign(environments.identity, {
    NVBES_IDENTITY_TOKEN_KEY_ID: nextKid,
    NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: next.publicKey,
    NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM: next.privateKey,
    NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS: overlap(
      oldKid,
      oldPublic,
      Math.floor(Date.now() / 1000) + 3600,
    ),
  });
  await restart('identity');
  const after = { account: await issue('account', false), billing: await issue('billing', false) };
  for (const token of Object.values(after))
    assert.equal(JSON.parse(Buffer.from(token.split('.')[0], 'base64url')).kid, nextKid);
  await checkPair(before, 200);
  await checkPair(after, 200);

  // Roll back after all consumers have already promoted the new key. Each
  // completed restart must preserve both generations, including new-key grants.
  for (const active of [
    {
      kid: nextKid,
      publicKey: next.publicKey,
      privateKey: next.privateKey,
      otherKid: oldKid,
      otherPublic: oldPublic,
    },
    {
      kid: oldKid,
      publicKey: oldPublic,
      privateKey: oldPrivate,
      otherKid: nextKid,
      otherPublic: next.publicKey,
    },
  ]) {
    for (const service of ['identity', 'account', 'billing']) {
      Object.assign(environments[service], {
        NVBES_IDENTITY_TOKEN_KEY_ID: active.kid,
        NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: active.publicKey,
        NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS: overlap(
          active.otherKid,
          active.otherPublic,
          Math.floor(Date.now() / 1000) + 3600,
        ),
      });
      if (service === 'identity')
        environments.identity.NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM = active.privateKey;
      await restart(service);
      await checkPair(before, 200);
      await checkPair(after, 200);
    }
    assert.deepEqual(await ids(), [oldKid, nextKid].sort(compareIds));
  }
  const rollback = {
    account: await issue('account', false),
    billing: await issue('billing', false),
  };
  for (const token of Object.values(rollback))
    assert.equal(JSON.parse(Buffer.from(token.split('.')[0], 'base64url')).kid, oldKid);
  await checkPair(rollback, 200);

  // Accelerated retirement deliberately occurs while the old JWTs are still unexpired.
  const retireAt = Math.floor(Date.now() / 1000) + 20;
  for (const service of ['identity', 'account', 'billing']) {
    Object.assign(environments[service], {
      NVBES_IDENTITY_TOKEN_KEY_ID: nextKid,
      NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: next.publicKey,
      NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS: overlap(oldKid, oldPublic, retireAt),
    });
    if (service === 'identity')
      environments.identity.NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM = next.privateKey;
    await restart(service);
  }
  assert.deepEqual(await ids(), [oldKid, nextKid].sort(compareIds));
  await checkPair(before, 200);
  await checkPair(after, 200);
  await checkPair(rollback, 200);
  while (Date.now() / 1000 < retireAt) await delay(100);
  for (const token of [...Object.values(before), ...Object.values(rollback)])
    assert.ok(JSON.parse(Buffer.from(token.split('.')[1], 'base64url')).exp > Date.now() / 1000);
  assert.deepEqual(await ids(), [nextKid]);
  await checkPair(before, 401);
  await checkPair(rollback, 401);
  await checkPair(after, 200);
}
