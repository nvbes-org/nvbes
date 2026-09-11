import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  mutationCheckout,
  mutationRunPaths,
  publishMutationReport as publish,
  verifyMutationArtifact,
} from './mutation-campaign-artifacts.mjs';
import { createMutationCheckpoint, executeMutationSteps } from './mutation-campaign-state.mjs';

const publishMutationReport = (options) => publish({ validate: () => {}, ...options });

function fixture(t) {
  const root = mkdtempSync(path.join(os.tmpdir(), 'nvbes-mutation-artifacts-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function reportAt(root, value) {
  const checkpoint = createMutationCheckpoint(root, 'test');
  const directory = path.relative(root, path.dirname(checkpoint.file)).split(path.sep).join('/');
  const { report } = mutationRunPaths('test', directory);
  writeFileSync(path.join(root, report), JSON.stringify(value));
  return report;
}

test('isolates reports and sandboxes while keeping direct Stryker usage compatible', () => {
  assert.deepEqual(mutationRunPaths('test'), {
    report: '.temp/typescript/test/mutation.json',
    sandbox: '.temp/stryker-test',
  });
  const first = mutationRunPaths('test', '.temp/mutation-runs/test-abc123');
  const second = mutationRunPaths('test', '.temp/mutation-runs/test-def456');
  assert.notEqual(first.report, second.report);
  assert.notEqual(first.sandbox, second.sandbox);
});

for (const directory of [
  '',
  '/tmp/test-abc123',
  '.temp/mutation-runs/other-abc123',
  '.temp/mutation-runs/test-abc123/../x',
]) {
  test(`rejects unsafe or mismatched run directory ${directory}`, () => {
    assert.throws(() => mutationRunPaths('test', directory), /Invalid/u);
  });
}

test('publishing a later campaign cannot overwrite the earlier archived report', (t) => {
  const root = fixture(t);
  const before = { sha: 'a'.repeat(40), checkoutSha256: 'b'.repeat(64), trackedChanges: false };
  const first = publishMutationReport({
    root,
    name: 'test',
    report: reportAt(root, { run: 1 }),
    before,
    after: before,
  });
  const second = publishMutationReport({
    root,
    name: 'test',
    report: reportAt(root, { run: 2 }),
    before,
    after: before,
  });
  assert.deepEqual(verifyMutationArtifact(root, first), { run: 1 });
  assert.deepEqual(verifyMutationArtifact(root, second), { run: 2 });
  assert.notEqual(first.sha256, second.sha256);
  assert.deepEqual(
    JSON.parse(readFileSync(path.join(root, '.temp/typescript/test/mutation.json'))),
    { run: 2 },
  );
});

for (const field of ['sha', 'checkoutSha256', 'trackedChanges']) {
  test(`refuses publication after ${field} changes`, (t) => {
    const root = fixture(t);
    const before = { sha: 'a', checkoutSha256: 'b', trackedChanges: false };
    assert.throws(
      () =>
        publishMutationReport({
          root,
          name: 'test',
          report: reportAt(root, {}),
          before,
          after: { ...before, [field]: 'changed' },
        }),
      /checkout changed/u,
    );
  });
}

test('rejects an altered report even when its byte length is unchanged', (t) => {
  const root = fixture(t);
  const artifact = publishMutationReport({
    root,
    name: 'test',
    report: reportAt(root, { run: 1 }),
    before: {},
    after: {},
  });
  writeFileSync(path.join(root, artifact.report), JSON.stringify({ run: 2 }));
  assert.throws(() => verifyMutationArtifact(root, artifact), /digest changed/u);
});

test('validates the exact published bytes and refuses a changed invalid report', (t) => {
  const root = fixture(t);
  const report = reportAt(root, { score: 89.99 });
  assert.throws(
    () =>
      publishMutationReport({
        root,
        name: 'test',
        report,
        before: {},
        after: {},
        validate: (value) => assert(value.score >= 90, 'insufficient score'),
      }),
    /insufficient/u,
  );
  assert.throws(() => readFileSync(path.join(root, '.temp/typescript/test/mutation.json')), {
    code: 'ENOENT',
  });
});

test('rejects artifact traversal and reports from another package', (t) => {
  const root = fixture(t);
  assert.throws(() => verifyMutationArtifact(root, { report: '../private.json' }), /Invalid/u);
  assert.throws(
    () =>
      publishMutationReport({
        root,
        name: 'other',
        report: reportAt(root, {}),
        before: {},
        after: {},
      }),
    /Invalid/u,
  );
});

test('cannot report passed when artifact finalization fails; errors stay redacted', () => {
  let final;
  const exit = executeMutationSteps({
    context: {},
    steps: [{ name: 'mutation', timeout: 100 }],
    run: () => ({ status: 0 }),
    persist: (state) => {
      final = structuredClone(state);
    },
    finalize: () => {
      throw new Error('secret');
    },
  });
  assert.equal(exit, 1);
  assert.equal(final.status, 'blocked');
  assert.equal(final.reason, 'artifact-finalization-error');
  assert.equal(final.releaseDecision, 'NO-GO');
  assert.ok(!JSON.stringify(final).includes('secret'));
});

test('successful finalization persists the exact artifact reference', () => {
  let final;
  const artifact = { report: 'report.json', sha256: 'a'.repeat(64), bytes: 2 };
  assert.equal(
    executeMutationSteps({
      context: {},
      steps: [{ name: 'mutation', timeout: 100 }],
      run: () => ({ status: 0 }),
      persist: (state) => {
        final = structuredClone(state);
      },
      finalize: () => artifact,
    }),
    0,
  );
  assert.deepEqual(final.artifact, artifact);
  assert.equal(final.releaseDecision, 'NO-GO');
});

test('checkout fingerprint detects tracked edits, untracked tests and commits', (t) => {
  const root = fixture(t);
  const git = (args) => execFileSync('git', args, { cwd: root, stdio: 'pipe' });
  git(['init']);
  git(['config', 'user.name', 'Synthetic Test']);
  git(['config', 'user.email', 'synthetic@example.test']);
  writeFileSync(path.join(root, 'tracked.txt'), 'first');
  git(['add', 'tracked.txt']);
  git(['-c', 'commit.gpgsign=false', 'commit', '-m', 'fixture']);
  const before = mutationCheckout(root);
  writeFileSync(path.join(root, 'tracked.txt'), 'second');
  assert.notEqual(mutationCheckout(root).checkoutSha256, before.checkoutSha256);
  assert.equal(mutationCheckout(root).trackedChanges, true);
  writeFileSync(path.join(root, 'tracked.txt'), 'first');
  mkdirSync(path.join(root, 'libs/ts/example/src'), { recursive: true });
  writeFileSync(path.join(root, 'libs/ts/example/src/new.test.ts'), 'test');
  assert.notEqual(mutationCheckout(root).checkoutSha256, before.checkoutSha256);
  git(['add', '.']);
  git(['-c', 'commit.gpgsign=false', 'commit', '-m', 'new test']);
  assert.notEqual(mutationCheckout(root).sha, before.sha);
});
