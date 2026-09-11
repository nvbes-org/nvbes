import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { createMutationCheckpoint, executeMutationSteps } from './mutation-campaign-state.mjs';

const steps = ['calibration', 'mutation', 'threshold-gate'].map((name) => ({ name, timeout: 100 }));
test('rejects an empty or duplicate campaign and unsafe package paths', () => {
  const options = { context: {}, run: () => ({ status: 0 }), persist: () => {} };
  assert.throws(() => executeMutationSteps({ ...options, steps: [] }), /empty/u);
  assert.throws(
    () => executeMutationSteps({ ...options, steps: [steps[0], steps[0]] }),
    /Duplicate/u,
  );
  assert.throws(() => createMutationCheckpoint('/unused', '../unsafe'), /Invalid/u);
});
function execute(run) {
  const snapshots = [];
  const exit = executeMutationSteps({
    context: { sha: 'a'.repeat(40), package: 'test' },
    steps,
    run,
    persist: (state) => snapshots.push(structuredClone(state)),
  });
  return { exit, snapshots, final: snapshots.at(-1) };
}

test('records each step before execution and retains NO-GO even on diagnostic success', () => {
  const result = execute(() => ({ status: 0 }));
  assert.equal(result.exit, 0);
  assert.equal(result.final.status, 'passed');
  assert.equal(result.final.releaseDecision, 'NO-GO');
  assert.ok(result.final.finishedAt);
  assert.deepEqual(
    result.final.steps.map((step) => step.status),
    ['passed', 'passed', 'passed'],
  );
  assert.deepEqual(
    result.snapshots.filter((state) => state.activeStep).map((state) => state.activeStep),
    ['calibration', 'mutation', 'threshold-gate'],
  );
  assert.equal(result.snapshots[0].finishedAt, null);
});

for (const [label, failure, status, reason] of [
  [
    'timeout',
    { error: { code: 'ETIMEDOUT', message: 'secret' }, signal: 'SIGTERM' },
    'blocked',
    'timeout',
  ],
  ['spawn error', { error: { code: 'ENOENT', message: 'secret' } }, 'blocked', 'execution-error'],
  ['signal', { signal: 'SIGTERM' }, 'blocked', 'signal'],
  ['nonzero', { status: 1 }, 'failed', 'nonzero-exit'],
  ['absent exit', { status: null }, 'failed', 'nonzero-exit'],
]) {
  test(`persists ${label}, keeps dependent steps not-run, and excludes sensitive details`, () => {
    let calls = 0;
    const result = execute(() => (++calls === 1 ? { status: 0 } : failure));
    assert.equal(result.exit, 1);
    assert.equal(calls, 2);
    assert.equal(result.final.status, status);
    assert.equal(result.final.steps[1].reason, reason);
    assert.equal(result.final.steps[2].status, 'not-run');
    assert.ok(!JSON.stringify(result.snapshots).includes('secret'));
  });
}

test('stops at a calibration exception and never reports mutation success', () => {
  const result = execute(() => {
    throw new Error('secret');
  });
  assert.equal(result.exit, 1);
  assert.deepEqual(
    result.final.steps.map((row) => row.status),
    ['blocked', 'not-run', 'not-run'],
  );
});

test('threshold refusal is not confused with successful mutation execution', () => {
  const result = execute(({ name }) => ({ status: name === 'threshold-gate' ? 1 : 0 }));
  assert.equal(result.exit, 1);
  assert.equal(result.final.steps[1].status, 'passed');
  assert.equal(result.final.status, 'failed');
});

test('real child timeout is recorded and checkpoint files are distinct and private', () => {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-mutation-state-'));
  try {
    const checkpoint = createMutationCheckpoint(directory, 'test');
    assert.notEqual(createMutationCheckpoint(directory, 'test').file, checkpoint.file);
    const exit = executeMutationSteps({
      context: { sha: 'b'.repeat(40) },
      steps,
      run: () =>
        spawnSync(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeout: 100 }),
      persist: checkpoint.persist,
    });
    assert.equal(exit, 1);
    const state = JSON.parse(readFileSync(checkpoint.file, 'utf8'));
    assert.equal(state.steps[0].reason, 'timeout');
    assert.equal(state.steps[1].status, 'not-run');
    assert.equal(state.sha, 'b'.repeat(40));
    assert.equal(statSync(checkpoint.file).mode & 0o777, 0o600);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
