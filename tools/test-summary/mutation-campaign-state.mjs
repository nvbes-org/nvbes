import { mkdirSync, mkdtempSync, renameSync, writeFileSync } from 'node:fs';
import path from 'node:path';

export function createMutationCheckpoint(root, name) {
  assert(/^[a-z][a-z0-9-]*$/u.test(name), 'Invalid mutation package name');
  const directory = path.join(root, '.temp', 'mutation-runs');
  mkdirSync(directory, { recursive: true });
  const runDirectory = mkdtempSync(path.join(directory, `${name}-`));
  const file = path.join(runDirectory, 'status.json');
  return {
    file,
    persist(state) {
      const pending = `${file}.pending`;
      writeFileSync(pending, `${JSON.stringify(state, null, 2)}\n`, { mode: 0o600 });
      renameSync(pending, file);
    },
  };
}

export function executeMutationSteps({
  context,
  steps,
  run,
  persist,
  finalize = () => undefined,
  now = () => new Date().toISOString(),
}) {
  assert(steps.length > 0, 'Mutation steps must not be empty');
  assert.equal(
    new Set(steps.map(({ name }) => name)).size,
    steps.length,
    'Duplicate mutation step',
  );
  const state = {
    ...context,
    schemaVersion: 1,
    purpose: 'Local execution diagnostic only; not authenticated release evidence.',
    releaseDecision: 'NO-GO',
    startedAt: now(),
    finishedAt: null,
    activeStep: null,
    status: 'not-run',
    steps: steps.map(({ name, timeout }) => ({ name, timeoutMs: timeout, status: 'not-run' })),
  };
  persist(state);
  for (const [index, step] of steps.entries()) {
    const row = state.steps[index];
    state.activeStep = step.name;
    row.startedAt = now();
    persist(state);
    let result;
    try {
      result = run(step);
    } catch {
      result = { error: { code: 'EXECUTION_EXCEPTION' } };
    }
    row.finishedAt = now();
    row.exitCode = Number.isInteger(result.status) ? result.status : null;
    row.signal = typeof result.signal === 'string' ? result.signal : null;
    // Never persist raw errors, command output or inherited environment values.
    row.reason =
      result.error?.code === 'ETIMEDOUT'
        ? 'timeout'
        : result.error
          ? 'execution-error'
          : result.signal
            ? 'signal'
            : result.status === 0
              ? null
              : 'nonzero-exit';
    row.status =
      result.error || result.signal ? 'blocked' : result.status === 0 ? 'passed' : 'failed';
    state.activeStep = null;
    persist(state);
    if (row.status !== 'passed') {
      state.status = row.status;
      state.finishedAt = now();
      persist(state);
      return 1;
    }
  }
  try {
    state.artifact = finalize();
    state.status = 'passed';
  } catch {
    state.status = 'blocked';
    state.reason = 'artifact-finalization-error';
  }
  state.finishedAt = now();
  persist(state);
  return state.status === 'passed' ? 0 : 1;
}
import assert from 'node:assert/strict';
