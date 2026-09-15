import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { profileOptions, thinkTimeSeconds } from '../load-tests/account/profiles.js';
import {
  validateResolvedTargetOrigin,
  validateTargetOrigin,
} from '../load-tests/account/target-policy.js';
import { validateLoadProfiles } from './check-load-suite.mjs';

const stagingPolicy = {
  allowedOrigins: 'https://account.staging.nvbes.fr',
  kind: 'service',
  productionOrigins: 'https://account.nvbes.fr,https://api.nvbes.fr',
  targetEnvironment: 'staging',
};

test('accepts only the exact staging origin', () => {
  assert.equal(
    validateTargetOrigin({
      ...stagingPolicy,
      target: 'https://account.staging.nvbes.fr',
    }),
    'https://account.staging.nvbes.fr',
  );
  assert.throws(() =>
    validateTargetOrigin({
      ...stagingPolicy,
      target: 'https://unrelated.example',
    }),
  );
  assert.throws(() =>
    validateTargetOrigin({
      ...stagingPolicy,
      target: 'https://account.staging.nvbes.fr:444',
    }),
  );
});

test('rejects credentials, encoding and production even when authority is misleading', () => {
  for (const target of [
    'https://account.nvbes.fr@account.staging.nvbes.fr',
    'https://account.nvbes.fr%40account.staging.nvbes.fr',
    'https://account.nvbes.fr:8443',
    'https://account.nvbes.fr.',
    'https://account.staging.nvbes.fr\\@169.254.169.254',
  ]) {
    assert.throws(() => validateTargetOrigin({ ...stagingPolicy, target }), target);
  }
});

test('rejects every known or protected production alias even if staging allowlists it', () => {
  for (const target of [
    'https://account.nvbes.fr',
    'https://account.nvbes.fr:8443',
    'https://api.nvbes.fr',
    'https://api.nvbes.fr:8443',
    'https://backoffice.nvbes.fr',
    'https://cloud.nvbes.fr',
    'https://account.primary.example',
  ]) {
    assert.throws(() =>
      validateTargetOrigin({
        allowedOrigins: target,
        kind: 'service',
        productionOrigins: 'https://api.nvbes.fr,https://account.primary.example',
        target,
        targetEnvironment: 'staging',
      }),
    );
  }
});

test('staging rejects a missing protected production-origin denylist', () => {
  assert.throws(
    () =>
      validateTargetOrigin({
        allowedOrigins: 'https://account.staging.nvbes.fr',
        kind: 'service',
        target: 'https://account.staging.nvbes.fr',
        targetEnvironment: 'staging',
      }),
    /protected production-origin denylist/u,
  );
});

test('rejects internal and arbitrary staging targets even if supplied by dispatch input', () => {
  for (const target of [
    'http://127.0.0.1:4000',
    'https://169.254.169.254',
    'https://10.0.0.1',
    'https://metadata.internal',
  ]) {
    assert.throws(() =>
      validateTargetOrigin({
        allowedOrigins: target,
        kind: 'service',
        target,
        targetEnvironment: 'staging',
      }),
    );
  }
});

test('allows only explicit loopback origins in ci', () => {
  assert.equal(
    validateTargetOrigin({
      kind: 'service',
      target: 'http://127.0.0.1:4000',
      targetEnvironment: 'ci',
    }),
    'http://127.0.0.1:4000',
  );
  assert.throws(() =>
    validateTargetOrigin({
      kind: 'service',
      target: 'http://192.168.1.10:4000',
      targetEnvironment: 'ci',
    }),
  );
});

test('resolves staging DNS and accepts only entirely public answer sets', async () => {
  const calls = [];
  assert.equal(
    await validateResolvedTargetOrigin({
      ...stagingPolicy,
      resolveHostname: async (hostname) => {
        calls.push(hostname);
        return [
          { address: '8.8.8.8', family: 4 },
          { address: '2606:4700:4700::1111', family: 6 },
        ];
      },
      target: 'https://account.staging.nvbes.fr',
    }),
    'https://account.staging.nvbes.fr',
  );
  assert.deepEqual(calls, ['account.staging.nvbes.fr']);
});

test('rejects every private, link-local or reserved DNS answer', async () => {
  for (const address of [
    '0.0.0.1',
    '10.0.0.1',
    '100.64.0.1',
    '169.254.169.254',
    '172.16.0.1',
    '192.0.2.1',
    '192.168.0.1',
    '198.18.0.1',
    '198.51.100.1',
    '203.0.113.1',
    '224.0.0.1',
    '240.0.0.1',
    '::1',
    'fc00::1',
    'fe80::1',
    'ff02::1',
    '2001:db8::1',
  ]) {
    await assert.rejects(
      validateResolvedTargetOrigin({
        ...stagingPolicy,
        resolveHostname: async () => [{ address }],
        target: 'https://account.staging.nvbes.fr',
      }),
      /non-public or reserved/u,
      address,
    );
  }
});

test('fails closed on mixed, empty, malformed and failed DNS resolution', async () => {
  for (const resolveHostname of [
    async () => [{ address: '8.8.8.8' }, { address: '127.0.0.1' }],
    async () => [],
    async () => [{ address: 'not-an-address' }],
    async () => {
      throw new Error('synthetic DNS failure');
    },
  ]) {
    await assert.rejects(
      validateResolvedTargetOrigin({
        ...stagingPolicy,
        resolveHostname,
        target: 'https://account.staging.nvbes.fr',
      }),
    );
  }
});

test('does not resolve allowlisted local targets', async () => {
  let calls = 0;
  assert.equal(
    await validateResolvedTargetOrigin({
      kind: 'service',
      resolveHostname: async () => {
        calls += 1;
        return [];
      },
      target: 'http://127.0.0.1:4000',
      targetEnvironment: 'ci',
    }),
    'http://127.0.0.1:4000',
  );
  assert.equal(calls, 0);
});

test('profiles reject inconsistent durations and invalid environment values', () => {
  assert.throws(() => profileOptions('stress', { ACCOUNT_K6_DURATION: '6m' }));
  assert.throws(() => profileOptions('volume', { ACCOUNT_K6_ITERATIONS: '0' }));
  assert.throws(() => profileOptions('unknown', {}));
  assert.throws(() => thinkTimeSeconds('-1', 1));
  assert.throws(() => thinkTimeSeconds('61', 1));
  assert.equal(thinkTimeSeconds('0.25', 1), 0.25);

  const load = profileOptions('load', {});
  assert.equal(load.maxRedirects, 0);
  assert.deepEqual(load.thresholds.dropped_iterations, ['count==0']);
  assert.match(load.thresholds.iterations[0], /^count>=\d+$/u);

  const volume = profileOptions('volume', { ACCOUNT_K6_ITERATIONS: '1234' });
  assert.deepEqual(volume.thresholds.iterations, ['count>=1234']);
});

test('all load profiles satisfy the exact executable contract', () => {
  assert.doesNotThrow(() => validateLoadProfiles());
});

test('profile contract rejects a deleted profile', () => {
  assert.throws(
    () =>
      validateLoadProfiles((profile, environment) =>
        profile === 'volume' ? undefined : profileOptions(profile, environment),
      ),
    /load profile volume options are missing/u,
  );
});

test('preflight and native/container k6 paths enforce the protected denylist', async () => {
  const sources = await Promise.all([
    readFile('tools/account-quality/validate-load-target.mjs', 'utf8'),
    readFile('tools/load-tests/account/account-api.js', 'utf8'),
    readFile('tools/load-tests/account/account-session.js', 'utf8'),
    readFile('tools/account-quality/run-account-k6.sh', 'utf8'),
    readFile(
      existsSync('.github/workflows/account-quality.yml')
        ? '.github/workflows/account-quality.yml'
        : '.github/workflows-archive/account-quality.yml',
      'utf8',
    ),
  ]);
  for (const source of sources) {
    assert.match(source, /ACCOUNT_PRODUCTION_DENIED_ORIGINS/u);
  }
});
