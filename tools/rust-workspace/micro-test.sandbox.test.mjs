import assert from 'node:assert/strict';
import test from 'node:test';
import { isolatedRun, networkSandbox, verifySandbox } from './micro-test.sandbox.mjs';

test('kernel sandbox permits execution and denies IPv4/IPv6 network operations', () => {
  const sandbox = networkSandbox();
  try {
    verifySandbox(sandbox);
    const result = isolatedRun(sandbox, process.execPath, [
      '-e',
      `
      const { spawnSync } = require('node:child_process');
      const child = spawnSync(process.execPath, ['-e',
        "require('node:net').createServer().listen(0, '127.0.0.1', () => process.exit(0))"]);
      process.exit(child.status === 0 ? 1 : 0);
    `,
    ]);
    assert.equal(result.status, 0, 'Child processes must inherit network denial');
  } finally {
    sandbox.cleanup();
  }
});
