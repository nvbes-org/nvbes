import assert from 'node:assert/strict';
import test from 'node:test';
import { emailTestArguments } from './micro-test.typescript.mjs';
import { networkSandbox, isolatedRun } from './micro-test.sandbox.mjs';

test('installed Email runner starts without registry access or runtime provisioning', () => {
  const sandbox = networkSandbox();
  try {
    const result = isolatedRun(sandbox, process.execPath, emailTestArguments(), {
      env: { PATH: process.env.PATH, TZ: 'UTC', LANG: 'C', LC_ALL: 'C' },
    });
    assert.equal(
      result.status,
      0,
      `Email runtime failed (${result.signal ?? result.error ?? ''})\n${result.stdout}\n${result.stderr}`,
    );
  } finally {
    sandbox.cleanup();
  }
});
