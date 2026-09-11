import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  assembleReceiptDraft,
  validateAssemblyRun,
  validateReceipt,
} from './v1-receipt-assembly.mjs';
import { suiteCommandDigest } from './v1-suite-receipt.mjs';

const sha = 'a'.repeat(40);
const suite = {
  id: 'platform.workspace-check',
  execution: 'automated',
  target: 'rust-workspace:check',
  cases: ['platform.workspace-check.complete-run'],
};
const domains = [{ domain: 'platform', suites: [suite] }];
function fixture() {
  const run = {
    id: 123,
    repository: { id: 1, full_name: 'nvbes-org/nvbes' },
    head_sha: sha,
    path: '.github/workflows/ci.yml',
    status: 'completed',
    conclusion: 'success',
    run_attempt: 1,
    event: 'workflow_dispatch',
    run_started_at: '2026-09-10T10:00:00.000Z',
    updated_at: '2026-09-10T11:00:00.000Z',
  };
  const receipt = {
    receiptVersion: 1,
    suite: suite.id,
    domain: 'platform',
    sha,
    environment: 'isolated',
    status: 'passed',
    startedAt: '2026-09-10T10:10:00.000Z',
    completedAt: '2026-09-10T10:20:00.000Z',
    attempt: 1,
    tools: { node: 'v24', pnpm: '11.18.0', nx: '23.1.1' },
    target: suite.target,
    commandSha256: suiteCommandDigest(suite.target),
    producer: {
      kind: 'github-actions',
      repository: 'nvbes-org/nvbes',
      workflow: run.path,
      runId: run.id,
    },
    cases: [{ id: suite.cases[0], status: 'passed' }],
  };
  const listing = {
    total_count: 2,
    artifacts: [
      { id: 456, name: `v1-receipt-${suite.id}-${sha}` },
      { id: 789, name: 'unrelated-cache' },
    ],
  };
  return { run, receipt, listing };
}

test('assembles authenticated receipts into an explicitly incomplete deterministic draft', () => {
  const f = fixture();
  const calls = [];
  const result = assembleReceiptDraft({
    run: f.run,
    runId: '123',
    listing: f.listing,
    domains,
    manifestDigest: 'b'.repeat(64),
    readArtifact: (input) => {
      calls.push(input);
      return Buffer.from(JSON.stringify(f.receipt));
    },
  });
  assert.equal(result.draft.incomplete, true);
  assert.equal(result.draft.results.length, 1);
  assert.equal(result.draft.results[0].receiptVersion, undefined);
  assert.deepEqual(result.draft.results[0].artifacts, ['receipts/platform.workspace-check.json']);
  assert.deepEqual(result.draft.measurements, []);
  assert.equal(result.draft.operations, null);
  assert.match(result.draft.artifacts[0].sha256, /^[a-f0-9]{64}$/u);
  assert.equal(
    result.files.get('receipts/platform.workspace-check.json').toString(),
    JSON.stringify(f.receipt),
  );
  assert.deepEqual(calls[0].reference, {
    artifactId: 456,
    member: 'platform.workspace-check.json',
  });
});

test('Nx forwards named arguments without shell interpolation', () => {
  const project = JSON.parse(readFileSync('tools/test-summary/project.json', 'utf8'));
  assert.equal(
    project.targets.receipt.options.command,
    'node tools/test-summary/v1-suite-receipt.mjs',
  );
  assert.equal(
    project.targets['assemble-receipts'].options.command,
    'node tools/test-summary/v1-receipt-assembly.mjs',
  );
});

for (const [name, change] of [
  [
    'wrong ID',
    (r) => {
      r.id = 124;
    },
  ],
  [
    'wrong repository',
    (r) => {
      r.repository.full_name = 'fork/nvbes';
    },
  ],
  [
    'missing repository ID',
    (r) => {
      delete r.repository.id;
    },
  ],
  [
    'invalid SHA',
    (r) => {
      r.head_sha = 'short';
    },
  ],
  [
    'unknown workflow',
    (r) => {
      r.path = '.github/workflows/release.yml';
    },
  ],
  [
    'running workflow',
    (r) => {
      r.status = 'in_progress';
    },
  ],
  [
    'failed workflow',
    (r) => {
      r.conclusion = 'failure';
    },
  ],
  [
    'rerun',
    (r) => {
      r.run_attempt = 2;
    },
  ],
  [
    'pull request',
    (r) => {
      r.event = 'pull_request';
    },
  ],
  [
    'invalid timestamps',
    (r) => {
      r.updated_at = 'unknown';
    },
  ],
  [
    'reversed timestamps',
    (r) => {
      r.updated_at = '2026-09-10T09:00:00.000Z';
    },
  ],
]) {
  test(`rejects run with ${String(name)}`, () => {
    const f = fixture();
    change(f.run);
    assert.throws(() => validateAssemblyRun(f.run, '123'));
  });
}

for (const [name, change] of [
  [
    'wrong version',
    (r) => {
      r.receiptVersion = 2;
    },
  ],
  [
    'wrong suite',
    (r) => {
      r.suite = 'platform.workspace-lint';
    },
  ],
  [
    'wrong domain',
    (r) => {
      r.domain = 'identity';
    },
  ],
  [
    'wrong SHA',
    (r) => {
      r.sha = 'c'.repeat(40);
    },
  ],
  [
    'wrong environment',
    (r) => {
      r.environment = 'production';
    },
  ],
  [
    'failed status',
    (r) => {
      r.status = 'failed';
    },
  ],
  [
    'retry',
    (r) => {
      r.attempt = 2;
    },
  ],
  [
    'wrong target',
    (r) => {
      r.target = 'rust-workspace:test';
    },
  ],
  [
    'wrong command',
    (r) => {
      r.commandSha256 = '0'.repeat(64);
    },
  ],
  [
    'missing tools',
    (r) => {
      r.tools = {};
    },
  ],
  [
    'blank tool',
    (r) => {
      r.tools.node = ' ';
    },
  ],
  [
    'noncanonical date',
    (r) => {
      r.startedAt = '2026-09-10T10:10:00Z';
    },
  ],
  [
    'before run',
    (r) => {
      r.startedAt = '2026-09-10T09:00:00.000Z';
    },
  ],
  [
    'after run',
    (r) => {
      r.completedAt = '2026-09-10T12:00:00.000Z';
    },
  ],
  [
    'reversed dates',
    (r) => {
      r.completedAt = '2026-09-10T10:05:00.000Z';
    },
  ],
  [
    'wrong producer',
    (r) => {
      r.producer.runId = 124;
    },
  ],
  [
    'missing case',
    (r) => {
      r.cases = [];
    },
  ],
  [
    'failed case',
    (r) => {
      r.cases[0].status = 'failed';
    },
  ],
]) {
  test(`rejects receipt with ${String(name)}`, () => {
    const f = fixture();
    change(f.receipt);
    assert.throws(() => validateReceipt(f.receipt, suite.id, domains, f.run));
  });
}

for (const [name, change] of [
  [
    'incomplete listing',
    (f) => {
      f.listing.total_count++;
    },
  ],
  [
    'too many artifacts',
    (f) => {
      f.listing.total_count = 101;
      f.listing.artifacts.length = 101;
    },
  ],
  [
    'invalid manifest digest',
    (f) => {
      f.manifestDigest = 'short';
    },
  ],
  [
    'no receipt',
    (f) => {
      f.listing = { total_count: 1, artifacts: [{ id: 1, name: 'cache' }] };
    },
  ],
  [
    'unknown suite',
    (f) => {
      f.listing.artifacts[0].name = `v1-receipt-unknown.suite-${sha}`;
    },
  ],
  [
    'invalid receipt JSON',
    (f) => {
      f.bytes = Buffer.from('{');
    },
  ],
  [
    'empty receipt',
    (f) => {
      f.bytes = Buffer.alloc(0);
    },
  ],
  [
    'duplicate artifact ID',
    (f) => {
      f.listing.artifacts = [
        f.listing.artifacts[0],
        { ...f.listing.artifacts[0], name: `v1-receipt-platform.workspace-lint-${sha}` },
      ];
      f.listing.total_count = 2;
    },
  ],
  [
    'duplicate suite',
    (f) => {
      f.listing.artifacts = [f.listing.artifacts[0], { ...f.listing.artifacts[0], id: 457 }];
      f.listing.total_count = 2;
    },
  ],
]) {
  test(`refuses assembly with ${String(name)}`, () => {
    const source = fixture();
    const f = {
      ...source,
      manifestDigest: 'b'.repeat(64),
      bytes: Buffer.from(JSON.stringify(source.receipt)),
    };
    change(f);
    assert.throws(() =>
      assembleReceiptDraft({
        run: f.run,
        runId: '123',
        listing: f.listing,
        domains,
        manifestDigest: f.manifestDigest,
        readArtifact: () => f.bytes,
      }),
    );
  });
}
