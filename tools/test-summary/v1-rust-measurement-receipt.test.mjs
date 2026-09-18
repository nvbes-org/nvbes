import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assembleRustMeasurements } from './v1-rust-measurement-assembly.mjs';
import { produceRustMeasurements } from './v1-rust-measurement-receipt.mjs';

const sha = 'a'.repeat(40);
const file = 'libs/rust/example/src/lib.rs';
const unit = {
  name: 'nvbes-example',
  language: 'rust',
  sourceRoot: 'libs/rust/example',
  sources: [file],
  thresholds: {},
};
const run = {
  id: 123,
  head_sha: sha,
  path: '.github/workflows/v1-testing.yml',
  run_started_at: '2026-09-11T10:00:00Z',
  updated_at: '2026-09-11T11:00:00Z',
};
const context = {
  sha,
  tools: { rust: '1.98.1', llvmCov: '0.8.7', cargoMutants: '27.1.0' },
  producer: {
    kind: 'github-actions',
    repository: 'nvbes-org/nvbes',
    workflow: '.github/workflows/v1-testing.yml',
    runId: 123,
  },
};
const llvmBytes = Buffer.from(
  JSON.stringify({
    type: 'llvm.coverage.json.export',
    version: '3.1.0',
    data: [
      {
        files: [
          {
            filename: file,
            summary: {
              lines: { count: 10, covered: 10 },
              branches: { count: 4, covered: 4 },
            },
          },
        ],
      },
    ],
  }),
);
const mutationBytes = Buffer.from(
  JSON.stringify({
    cargo_mutants_version: '27.1.0',
    start_time: '2026-09-11T10:00:00Z',
    end_time: '2026-09-11T10:30:00Z',
    total_mutants: 1,
    outcomes: [
      {
        scenario: 'Baseline',
        summary: 'Success',
        phase_results: [
          { phase: 'Build', process_status: 'Success' },
          { phase: 'Test', process_status: 'Success' },
        ],
      },
      {
        scenario: {
          Mutant: {
            package: unit.name,
            file,
            replacement: 'false',
            span: { start: { line: 1, column: 1 }, end: { line: 1, column: 2 } },
          },
        },
        summary: 'CaughtMutant',
        phase_results: [
          { phase: 'Build', process_status: 'Success' },
          { phase: 'Test', process_status: { Failure: 101 } },
        ],
      },
    ],
  }),
);

function fixture() {
  const measurements = produceRustMeasurements({
    units: [unit],
    context,
    llvmBytes,
    mutationBytes,
    completedAt: '2026-09-11T10:31:00Z',
  });
  return new Map([
    ['measurements.json', Buffer.from(JSON.stringify(measurements))],
    ['llvm.json', llvmBytes],
    ['mutation.json', mutationBytes],
  ]);
}

test('produces and assembles exhaustive Rust measurements from raw reports', () => {
  const members = fixture();
  const assembled = assembleRustMeasurements({
    metadata: { id: 456 },
    run,
    units: [unit],
    readArtifact: ({ reference }) => members.get(reference.member),
  });
  assert.equal(assembled.measurements.length, 1);
  assert.equal(assembled.measurements[0].lines, 100);
  assert.equal(assembled.measurements[0].branches, 100);
  assert.equal(assembled.measurements[0].mutation, 100);
  assert.deepEqual(
    assembled.artifacts.map((artifact) => artifact.ci),
    [
      { artifactId: 456, member: 'measurements.json' },
      { artifactId: 456, member: 'llvm.json' },
      { artifactId: 456, member: 'mutation.json' },
    ],
  );
});

test('refuses a Rust metric below the V1 floor and an incomplete unit manifest', () => {
  const weak = JSON.parse(llvmBytes.toString('utf8'));
  weak.data[0].files[0].summary.lines.covered = 8;
  assert.throws(
    () =>
      produceRustMeasurements({
        units: [unit],
        context,
        llvmBytes: Buffer.from(JSON.stringify(weak)),
        mutationBytes,
        completedAt: '2026-09-11T10:31:00Z',
      }),
    /lines 80 \(minimum 90%\)/u,
  );
  const members = fixture();
  members.set('measurements.json', Buffer.from('[]'));
  assert.throws(
    () =>
      assembleRustMeasurements({
        metadata: { id: 456 },
        run,
        units: [unit],
        readArtifact: ({ reference }) => members.get(reference.member),
      }),
    /Incomplete Rust measurement manifest/u,
  );
});
