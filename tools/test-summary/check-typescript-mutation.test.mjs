import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mutationScore } from './check-typescript-mutation.mjs';

const report = (...statuses) => ({
  files: { 'source.ts': { mutants: statuses.map((status, id) => ({ id: String(id), status })) } },
});

test('counts timeouts and uncovered mutants as missed', () => {
  assert.deepEqual(
    mutationScore(report('Killed', 'Timeout', 'NoCoverage', 'Survived', 'CompileError')),
    { caught: 1, missed: 3, unviable: 1, score: 25 },
  );
});

test('rejects empty, ignored, duplicate and incomplete reports', () => {
  for (const candidate of [
    {},
    report(),
    report('CompileError'),
    report('Pending'),
    report('Ignored'),
    report('RuntimeError'),
  ])
    assert.throws(() => mutationScore(candidate));
  const candidate = report('Killed', 'Killed');
  candidate.files['source.ts'].mutants[1].id = '0';
  assert.throws(() => mutationScore(candidate), /Duplicate/u);
});
