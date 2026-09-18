import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { assembleTypescriptMeasurement } from './v1-measurement-assembly.mjs';
import { produceTypescriptMeasurement } from './v1-measurement-receipt.mjs';
import { assembleReceiptDraft } from './v1-receipt-assembly.mjs';

const sha = 'a'.repeat(40);
const source = 'export const answer = () => 42;\n';
const unit = {
  name: '@nvbes/example',
  language: 'typescript',
  sourceFiles: {
    'libs/ts/example/src/index.ts': {
      runtime: true,
      sha256: createHash('sha256').update(source).digest('hex'),
    },
  },
};
const run = {
  id: 123,
  repository: { id: 1, full_name: 'nvbes-org/nvbes' },
  head_sha: sha,
  path: '.github/workflows/ci.yml',
  status: 'completed',
  conclusion: 'success',
  run_attempt: 1,
  event: 'workflow_dispatch',
  run_started_at: '2026-09-10T10:00:00Z',
  updated_at: '2026-09-10T11:00:00Z',
};
const coverageBytes = Buffer.from(
  JSON.stringify({
    'libs/ts/example/src/index.ts': {
      lines: { total: 10, covered: 9, skipped: 0 },
      branches: { total: 4, covered: 4, skipped: 0 },
    },
  }),
);
const mutationBytes = Buffer.from(
  JSON.stringify({
    schemaVersion: '1.0',
    files: {
      'libs/ts/example/src/index.ts': {
        source,
        mutants: [{ id: '1', status: 'Killed' }],
      },
    },
  }),
);

function fixture() {
  const receipt = produceTypescriptMeasurement({
    unit,
    context: {
      sha,
      producer: {
        kind: 'github-actions',
        repository: 'nvbes-org/nvbes',
        workflow: run.path,
        runId: run.id,
      },
    },
    coverageBytes,
    mutationBytes,
    completedAt: '2026-09-10T10:30:00.000Z',
    tools: { node: 'v24', vitest: '4.1.10', stryker: '9.4.0' },
  });
  const members = new Map([
    ['measurement.json', Buffer.from(JSON.stringify(receipt))],
    ['coverage.json', coverageBytes],
    ['mutation.json', mutationBytes],
  ]);
  const metadata = {
    id: 456,
    name: `v1-measurement-typescript-example-${sha}`,
  };
  return { receipt, members, metadata };
}

test('assembles raw TypeScript reports with exact CI member references', () => {
  const f = fixture();
  const assembled = assembleTypescriptMeasurement({
    metadata: f.metadata,
    slug: 'example',
    run,
    units: [unit],
    readArtifact: ({ reference }) => f.members.get(reference.member),
  });
  assert.equal(assembled.unit, unit.name);
  assert.equal(assembled.measurement.lines, 90);
  assert.deepEqual(assembled.measurement.reports, {
    'istanbul-summary': 'measurements/typescript-example/coverage.json',
    stryker: 'measurements/typescript-example/mutation.json',
  });
  assert.deepEqual(
    assembled.artifacts.map((artifact) => artifact.ci),
    [
      { artifactId: 456, member: 'measurement.json' },
      { artifactId: 456, member: 'coverage.json' },
      { artifactId: 456, member: 'mutation.json' },
    ],
  );
});

test('measurement-only assembly remains an incomplete non-release draft', () => {
  const f = fixture();
  const listing = { total_count: 1, artifacts: [f.metadata] };
  const assembled = assembleReceiptDraft({
    run,
    runId: '123',
    listing,
    domains: [],
    units: [unit],
    manifestDigest: 'b'.repeat(64),
    readArtifact: ({ reference }) => f.members.get(reference.member),
  });
  assert.equal(assembled.draft.incomplete, true);
  assert.deepEqual(assembled.draft.results, []);
  assert.equal(assembled.draft.measurements.length, 1);
  assert.equal(assembled.files.size, 3);
});

test('refuses mismatched units, declared scores and incomplete raw reports', () => {
  for (const mutate of [
    (f) => {
      f.receipt.unit = '@nvbes/other';
    },
    (f) => {
      f.receipt.lines = 100;
    },
    (f) => {
      f.members.delete('mutation.json');
    },
  ]) {
    const f = fixture();
    mutate(f);
    f.members.set('measurement.json', Buffer.from(JSON.stringify(f.receipt)));
    assert.throws(() =>
      assembleTypescriptMeasurement({
        metadata: f.metadata,
        slug: 'example',
        run,
        units: [unit],
        readArtifact: ({ reference }) => f.members.get(reference.member),
      }),
    );
  }
});
