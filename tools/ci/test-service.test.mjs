import assert from 'node:assert/strict';
import test from 'node:test';
import { serviceName, serviceNetwork } from './test-service.mjs';

test('database names isolate runs, attempts and jobs', () => {
  const env = { GITHUB_RUN_ID: '123', GITHUB_RUN_ATTEMPT: '1', GITHUB_JOB: 'database' };
  const name = serviceName('postgres', env);
  for (const change of [
    { GITHUB_RUN_ID: '124' },
    { GITHUB_RUN_ATTEMPT: '2' },
    { GITHUB_JOB: 'rust' },
  ]) {
    assert.notEqual(serviceName('postgres', { ...env, ...change }), name);
  }
  assert.throws(() => serviceName('unsupported', env));
  assert.throws(() => serviceName('postgres', {}));
  assert.throws(() => serviceName('postgres', { ...env, GITHUB_JOB: '../unsafe' }));
});

test('hosted services use ephemeral loopback ports; Docker runners use network DNS', () => {
  assert.deepEqual(serviceNetwork('isolated', 5432), {
    args: ['--publish', '127.0.0.1::5432'],
    host: '127.0.0.1',
  });
  assert.deepEqual(serviceNetwork('isolated', 5432, 'runner-network'), {
    args: ['--network', 'runner-network'],
    host: 'isolated',
    port: 5432,
  });
});
