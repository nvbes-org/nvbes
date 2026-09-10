import assert from 'node:assert/strict';
import { availableParallelism } from 'node:os';
import test from 'node:test';

test('bounds mutation and test parallelism without changing scope, retries or score accounting', async () => {
  const previous = process.env.NVBES_MUTATION_PACKAGE;
  process.env.NVBES_MUTATION_PACKAGE = 'identity-sdk-web';
  try {
    const { default: config } = await import('./stryker.config.mjs');
    assert.equal(
      config.concurrency,
      Math.max(1, Math.min(4, Math.floor(availableParallelism() / 2))),
    );
    assert.match(
      config.commandRunner.command,
      /--retry=0 --pool=threads --maxWorkers=2 --bail=1$/u,
    );
    assert.equal(config.incremental, false);
    assert.equal(config.coverageAnalysis, 'off');
    assert.equal(config.timeoutMS, 10000);
    assert.deepEqual(config.thresholds, { high: 90, low: 90, break: 0 });
    assert.deepEqual(config.mutate, [
      'libs/ts/identity-sdk-web/src/**/*.{ts,tsx}',
      '!libs/ts/identity-sdk-web/src/**/*.{test,spec,gen,d}.{ts,tsx}',
    ]);
  } finally {
    if (previous === undefined) delete process.env.NVBES_MUTATION_PACKAGE;
    else process.env.NVBES_MUTATION_PACKAGE = previous;
  }
});
