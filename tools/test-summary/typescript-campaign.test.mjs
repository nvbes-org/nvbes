import assert from 'node:assert/strict';
import { test } from 'node:test';
import { runTypescriptCampaign } from './typescript-campaign.mjs';

const units = [{ name: '@nvbes/first' }, { name: '@nvbes/second' }];

test('full campaign continues independent measurements after a failed unit', async () => {
  const calls = [];
  const result = await runTypescriptCampaign(units, {
    run: async (unit, kind) => {
      calls.push([unit, kind]);
      return unit === '@nvbes/first' ? 1 : 0;
    },
  });
  assert.equal(calls.length, 4);
  assert.equal(result.passed, false);
  assert.deepEqual(result.results, [
    { unit: '@nvbes/first', coverage: 'failed', mutation: 'failed' },
    { unit: '@nvbes/second', coverage: 'passed', mutation: 'passed' },
  ]);
});

test('coverage-only never claims complete quality and checkpoints missing mutations', async () => {
  const checkpoints = [];
  const result = await runTypescriptCampaign(units, {
    coverageOnly: true,
    run: async () => 0,
    checkpoint: (results) => checkpoints.push(structuredClone(results)),
  });
  assert.equal(result.passed, false);
  assert.equal(checkpoints.length, 3);
  assert(checkpoints[0].every((row) => row.coverage === 'not-run'));
  assert(result.results.every((row) => row.coverage === 'passed' && row.mutation === 'not-run'));
});

test('execution exceptions are blocked and cannot disappear from the result', async () => {
  const result = await runTypescriptCampaign(units, {
    run: async () => {
      throw new Error('unavailable');
    },
  });
  assert.equal(result.passed, false);
  assert(result.results.every((row) => row.coverage === 'blocked' && row.mutation === 'blocked'));
});

test('requires every metric for every unique unit', async () => {
  assert.equal((await runTypescriptCampaign(units, { run: async () => 0 })).passed, true);
  await assert.rejects(runTypescriptCampaign([], { run: async () => 0 }));
  await assert.rejects(runTypescriptCampaign([units[0], units[0]], { run: async () => 0 }));
});
