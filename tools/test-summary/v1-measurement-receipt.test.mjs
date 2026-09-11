import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parse } from 'yaml';
import { typescriptUnits } from './v1-catalogue.mjs';
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
    /@nvbes\/example: lines 90% < 90.01%/u,
  );
});

test('manual trusted CI publishes bounded TypeScript measurement artifacts', () => {
  const workflow = parse(readFileSync('.github/workflows/v1-testing.yml', 'utf8'));
  assert.deepEqual(Object.keys(workflow.on), ['workflow_dispatch']);
  assert.deepEqual(
    Object.keys(workflow.on.workflow_dispatch.inputs).sort((a, b) => a.localeCompare(b)),
    ['expected_sha', 'fallback_of', 'runner'],
  );
  assert.equal(workflow.on.workflow_dispatch.inputs.runner.default, 'github-hosted');
  assert.equal(workflow.concurrency['cancel-in-progress'], true);
  assert.deepEqual(workflow.permissions, { actions: 'read', contents: 'read' });
  const job = workflow.jobs['typescript-measurement'];
  assert.equal(job['timeout-minutes'], 40);
  assert.equal(job.environment.name, 'ci-no-secrets');
  assert.deepEqual(job.strategy, {
    'fail-fast': false,
    'max-parallel': 1,
    matrix: {
      include: [
        { package: 'http-client', unit: '@nvbes/http-client' },
        { package: 'billing-client', unit: '@nvbes/billing-client' },
        { package: 'account-client', unit: '@nvbes/account-client' },
        { package: 'identity-sdk', unit: '@nvbes/identity-sdk' },
        { package: 'identity-client', unit: '@nvbes/identity-client' },
        { package: 'identity-sdk-web', unit: '@nvbes/identity-sdk-web' },
        { package: 'web-runtime', unit: '@nvbes/web-runtime' },
        { package: 'web-ui', unit: '@nvbes/web-ui' },
        { package: 'email-ui', unit: '@nvbes/email-ui' },
      ],
    },
  });
  assert.deepEqual(job.env, {
    NVBES_COVERAGE_PACKAGE: '${{ matrix.package }}',
    NVBES_MUTATION_PACKAGE: '${{ matrix.package }}',
  });
  const manifest = JSON.parse(readFileSync('docs/testing/v1/manifest.json', 'utf8'));
  assert.deepEqual(
    job.strategy.matrix.include.map((entry) => entry.unit).sort((a, b) => a.localeCompare(b)),
    typescriptUnits(manifest, process.cwd())
      .map((entry) => entry.name)
      .sort((a, b) => a.localeCompare(b)),
    'The CI campaign must include every applicable production package',
  );
  const mutations = job.steps.find((step) => step.id === 'mutation');
  assert.equal(mutations.if, "${{ !cancelled() && steps.setup.outcome == 'success' }}");
  const diagnostics = job.steps.find((step) => step.name === 'Publish TypeScript diagnostics');
  assert.equal(diagnostics.if, mutations.if);
  assert.equal(diagnostics.with['retention-days'], 7);
  assert.match(diagnostics.with.name, /^v1-diagnostic-/u);
  assert.equal(
    job.steps.find((step) => step.name === 'Produce TypeScript measurement receipt').if,
    undefined,
  );
  assert(job.steps.every((step) => step['continue-on-error'] !== true));
  const security = job.steps.findIndex((step) => step.name === 'CI/CD security gate');
  const setup = job.steps.findIndex((step) => step.uses === './.github/actions/ci-setup');
  assert.ok(security >= 0 && security < setup);
  const upload = job.steps.find((step) => step.name === 'Publish TypeScript measurement');
  assert.equal(upload.uses, 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02');
  assert.deepEqual(upload.with, {
    name: 'v1-measurement-typescript-${{ matrix.package }}-${{ github.sha }}',
    path: '.temp/v1-measurements/typescript-${{ matrix.package }}',
    'if-no-files-found': 'error',
    'retention-days': 7,
  });
});
