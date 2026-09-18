import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parse } from 'yaml';
import { ciContext, eligibleSuite, produceSuiteReceipt } from './v1-suite-receipt.mjs';

const suite = {
  id: 'platform.workspace-check',
  execution: 'automated',
  target: 'rust-workspace:check',
  cases: ['platform.workspace-check.complete-run'],
};
const domains = [{ domain: 'platform', suites: [suite] }];
const env = {
  GITHUB_SHA: 'a'.repeat(40),
  GITHUB_RUN_ID: '123',
  GITHUB_RUN_ATTEMPT: '1',
  GITHUB_REPOSITORY: 'nvbes-org/nvbes',
  GITHUB_EVENT_NAME: 'workflow_dispatch',
  GITHUB_WORKFLOW_REF: 'nvbes-org/nvbes/.github/workflows/ci.yml@refs/heads/main',
};

test('derives producer identity only from a trusted first-attempt GitHub context', () => {
  assert.deepEqual(ciContext(env), {
    sha: env.GITHUB_SHA,
    producer: {
      kind: 'github-actions',
      repository: env.GITHUB_REPOSITORY,
      workflow: '.github/workflows/ci.yml',
      runId: 123,
    },
  });
});

for (const [name, key, value] of [
  ['short SHA', 'GITHUB_SHA', 'a'.repeat(39)],
  ['invalid run', 'GITHUB_RUN_ID', '../123'],
  ['unsafe integer run', 'GITHUB_RUN_ID', '9007199254740992'],
  ['rerun', 'GITHUB_RUN_ATTEMPT', '2'],
  ['fork repository', 'GITHUB_REPOSITORY', 'fork/nvbes'],
  ['pull request', 'GITHUB_EVENT_NAME', 'pull_request'],
  ['unknown workflow', 'GITHUB_WORKFLOW_REF', 'nvbes-org/nvbes/.github/workflows/release.yml@main'],
  ['foreign workflow reference', 'GITHUB_WORKFLOW_REF', 'fork/nvbes/.github/workflows/ci.yml@main'],
  ['workflow without ref', 'GITHUB_WORKFLOW_REF', 'nvbes-org/nvbes/.github/workflows/ci.yml'],
]) {
  test(`refuses ${String(name)}`, () => assert.throws(() => ciContext({ ...env, [key]: value })));
}

test('accepts only automated Nx suites with one honest complete-run case', () => {
  assert.equal(eligibleSuite(domains, suite.id).suite.target, 'rust-workspace:check');
  assert.throws(() => eligibleSuite(domains, 'unknown'));
  assert.throws(() =>
    eligibleSuite([{ domain: 'platform', suites: [{ ...suite, execution: 'human' }] }], suite.id),
  );
  assert.throws(() =>
    eligibleSuite(
      [{ domain: 'platform', suites: [{ ...suite, cases: ['business.case'] }] }],
      suite.id,
    ),
  );
  assert.throws(() =>
    eligibleSuite(
      [{ domain: 'platform', suites: [{ ...suite, target: 'docs/runbook.md' }] }],
      suite.id,
    ),
  );
});

test('runs the exact manifest target and emits a passed receipt without command arguments', () => {
  const calls = [];
  const times = [new Date('2026-09-10T10:00:00Z'), new Date('2026-09-10T10:01:00Z')];
  const { result, passed } = produceSuiteReceipt({
    domains,
    suiteId: suite.id,
    context: ciContext(env),
    execute: (command) => {
      calls.push(command);
      return { status: 0, signal: null };
    },
    clock: () => times.shift(),
    tools: { node: 'v24.0.0', pnpm: '11.18.0', nx: '23.1.1' },
  });
  assert.equal(passed, true);
  assert.deepEqual(calls, [['pnpm', 'exec', 'nx', 'run', 'rust-workspace:check']]);
  assert.equal(result.status, 'passed');
  assert.equal(result.cases[0].status, 'passed');
  assert.equal(result.startedAt, '2026-09-10T10:00:00.000Z');
  assert.equal(result.completedAt, '2026-09-10T10:01:00.000Z');
  assert.match(result.commandSha256, /^[a-f0-9]{64}$/u);
  assert.equal(Object.hasOwn(result, 'command'), false);
});

for (const { name, execution } of [
  { name: 'nonzero exit', execution: { status: 1, signal: null } },
  { name: 'signal', execution: { status: null, signal: 'SIGTERM' } },
  { name: 'spawn error', execution: { status: null, signal: null, error: new Error('missing') } },
]) {
  test(`records ${name} as failed evidence`, () => {
    const { result, passed } = produceSuiteReceipt({
      domains,
      suiteId: suite.id,
      context: ciContext(env),
      execute: () => execution,
      clock: () => new Date('2026-09-10T10:00:00Z'),
      tools: { node: 'v24' },
    });
    assert.equal(passed, false);
    assert.equal(result.status, 'failed');
    assert.equal(result.cases[0].status, 'failed');
  });
}

test('refuses absent or blank tool versions', () => {
  const input = {
    domains,
    suiteId: suite.id,
    context: ciContext(env),
    execute: () => ({ status: 0 }),
    clock: () => new Date(),
    tools: {},
  };
  assert.throws(() => produceSuiteReceipt(input), /Tool versions/u);
  input.tools = { node: ' ' };
  assert.throws(() => produceSuiteReceipt(input), /Tool versions/u);
});

test('refuses a clock moving backwards', () => {
  const times = [new Date('2026-09-10T10:01:00Z'), new Date('2026-09-10T10:00:00Z')];
  assert.throws(
    () =>
      produceSuiteReceipt({
        domains,
        suiteId: suite.id,
        context: ciContext(env),
        execute: () => ({ status: 0, signal: null }),
        clock: () => times.shift(),
        tools: { node: 'v24' },
      }),
    /timestamps/u,
  );
});

test('manual trusted CI publishes the exact generated receipt with bounded retention', () => {
  const workflow = parse(readFileSync('.github/workflows/ci.yml', 'utf8'));
  const steps = workflow.jobs.rust.steps;
  const produce = steps.find((step) => step.name === 'Produce V1 workspace check receipt');
  const upload = steps.find((step) => step.name === 'Upload V1 workspace check receipt');
  assert.equal(produce.if, "github.event_name == 'workflow_dispatch'");
  assert.equal(
    produce.run,
    'node tools/test-summary/v1-suite-receipt.mjs --suite=platform.workspace-check',
  );
  assert.equal(upload.if, "always() && github.event_name == 'workflow_dispatch'");
  assert.equal(upload.uses, 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a');
  assert.deepEqual(upload.with, {
    name: 'v1-receipt-platform.workspace-check-${{ github.sha }}',
    path: '.temp/v1-receipts/platform.workspace-check.json',
    'include-hidden-files': true,
    'if-no-files-found': 'error',
    'retention-days': 7,
  });
  assert(steps.indexOf(produce) > steps.findIndex((step) => step.name === 'Validate rust'));
  assert(steps.indexOf(upload) > steps.indexOf(produce));
});
