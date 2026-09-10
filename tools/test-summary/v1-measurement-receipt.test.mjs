import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parse } from 'yaml';
import {
  produceTypescriptMeasurement,
  typescriptMeasurementSlug,
} from './v1-measurement-receipt.mjs';

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
const coverage = {
  'libs/ts/example/src/index.ts': {
    lines: { total: 10, covered: 9, skipped: 0 },
    branches: { total: 4, covered: 4, skipped: 0 },
  },
};
const mutation = {
  schemaVersion: '1.0',
  files: {
    'libs/ts/example/src/index.ts': {
      source,
      mutants: [{ id: '1', status: 'Killed' }],
    },
  },
};
const input = () => ({
  unit,
  context: {
    sha: 'a'.repeat(40),
    producer: {
      kind: 'github-actions',
      repository: 'nvbes-org/nvbes',
      workflow: '.github/workflows/ci.yml',
      runId: 123,
    },
  },
  coverageBytes: Buffer.from(JSON.stringify(coverage)),
  mutationBytes: Buffer.from(JSON.stringify(mutation)),
  completedAt: '2026-09-10T12:00:00.000Z',
  tools: { node: 'v24', vitest: '4.1.10', stryker: '9.4.0' },
  thresholds: { lines: 90, branches: 90, mutation: 90 },
});

test('produces TypeScript scores only from complete raw reports', () => {
  const measurement = produceTypescriptMeasurement(input());
  assert.equal(typescriptMeasurementSlug(unit), 'example');
  assert.equal(measurement.lines, 90);
  assert.equal(measurement.branches, 100);
  assert.equal(measurement.mutation, 100);
  assert.deepEqual(measurement.artifacts, ['measurement.json', 'coverage.json', 'mutation.json']);
});

test('refuses missing tools and raw reports outside candidate scope', () => {
  const missingTools = input();
  missingTools.tools = {};
  assert.throws(() => produceTypescriptMeasurement(missingTools), /Tool versions/u);
  const incomplete = input();
  incomplete.coverageBytes = Buffer.from('{}');
  assert.throws(() => produceTypescriptMeasurement(incomplete), /Missing runtime source/u);
  assert.throws(() => typescriptMeasurementSlug({ name: 'nvbes-example' }));
});

test('refuses a raw score below the applicable threshold', () => {
  const belowThreshold = input();
  belowThreshold.thresholds.lines = 90.01;
  assert.throws(
    () => produceTypescriptMeasurement(belowThreshold),
    /@nvbes\/example lines 90% < 90.01%/u,
  );
});

test('manual trusted CI publishes a bounded HTTP client measurement artifact', () => {
  const workflow = parse(readFileSync('.github/workflows/ci.yml', 'utf8'));
  const job = workflow.jobs['typescript-measurement'];
  assert.equal(job.if, "github.event_name == 'workflow_dispatch'");
  assert.equal(job['timeout-minutes'], 40);
  assert.equal(job.environment.name, 'ci-no-secrets');
  const upload = job.steps.find((step) => step.name === 'Publish HTTP client measurement');
  assert.equal(upload.uses, 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02');
  assert.deepEqual(upload.with, {
    name: 'v1-measurement-typescript-http-client-${{ github.sha }}',
    path: '.temp/v1-measurements/typescript-http-client',
    'if-no-files-found': 'error',
    'retention-days': 7,
  });
});
