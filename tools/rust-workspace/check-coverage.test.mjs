import assert from 'node:assert/strict';
import { test } from 'node:test';
import { evaluateCoverage } from './check-coverage.mjs';

function examplePackages() {
  return [
    { name: 'nvbes-core', manifest_path: '/repo/libs/rust/core/Cargo.toml' },
    {
      name: 'nvbes-platform',
      manifest_path: '/repo/libs/rust/platform/Cargo.toml',
    },
    { name: 'nvbes-email', manifest_path: '/repo/libs/rust/email/Cargo.toml' },
    { name: 'nvbes-scan', manifest_path: '/repo/libs/rust/scan/Cargo.toml' },
  ];
}

function fileEntry(filename, lines, functions, regions) {
  return {
    filename,
    summary: {
      lines: { count: lines, covered: Math.floor(lines * 0.9) },
      functions: { count: functions, covered: Math.floor(functions * 0.9) },
      regions: { count: regions, covered: Math.floor(regions * 0.9) },
    },
  };
}

function exampleConfig() {
  return {
    schemaVersion: 1,
    crates: {
      'nvbes-email': { threshold: { lines: 85, functions: 80, regions: 80 } },
      'nvbes-core': { threshold: { lines: 85, functions: 80, regions: 80 } },
      'nvbes-platform': {
        threshold: { lines: 85, functions: 80, regions: 80 },
      },
    },
    excluded: [{ crate: 'nvbes-scan', reason: 'skeleton without tests' }],
  };
}

test('aggregates per-crate coverage from the JSON report and passes', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 10, 90),
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 100, 10, 90),
          fileEntry('/repo/libs/rust/platform/src/identity.db.rs', 100, 10, 90),
        ],
      },
    ],
  };
  const result = evaluateCoverage(report, examplePackages(), exampleConfig());
  assert.deepEqual(result.failures, []);
  assert.deepEqual(result.orphans, []);
  assert.equal(result.rows.length, 3);
});

test('detects a coverage gap when a crate falls below its threshold', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 10, 90),
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 10, 2, 5),
          fileEntry('/repo/libs/rust/platform/src/identity.db.rs', 100, 10, 90),
        ],
      },
    ],
  };
  const result = evaluateCoverage(report, examplePackages(), exampleConfig());
  assert.ok(result.failures.some((failure) => failure.startsWith('nvbes-core')));
});

test('flags a workspace member without threshold or exclusion', () => {
  const report = {
    data: [{ files: [] }],
  };
  const config = {
    schemaVersion: 1,
    crates: {
      'nvbes-email': { threshold: { lines: 0, functions: 0, regions: 0 } },
    },
    excluded: [{ crate: 'nvbes-scan', reason: 'skeleton without tests' }],
  };
  const result = evaluateCoverage(report, examplePackages(), config);
  assert.ok(result.failures.some((failure) => failure.includes('nvbes-core')));
  assert.ok(result.failures.some((failure) => failure.includes('nvbes-platform')));
  assert.ok(!result.failures.some((failure) => failure.includes('nvbes-scan')));
});

test('rejects a configured crate that is not a workspace member', () => {
  const report = { data: [{ files: [] }] };
  const config = {
    schemaVersion: 1,
    crates: {
      'nvbes-ghost': { threshold: { lines: 0, functions: 0, regions: 0 } },
    },
    excluded: [],
  };
  assert.throws(
    () => evaluateCoverage(report, examplePackages(), config),
    /not a workspace member/u,
  );
});

test('rejects an excluded crate without a justification', () => {
  const report = { data: [{ files: [] }] };
  const config = {
    schemaVersion: 1,
    crates: {},
    excluded: [{ crate: 'nvbes-scan', reason: '' }],
  };
  assert.throws(() => evaluateCoverage(report, examplePackages(), config), /explicit reason/u);
});

test('reports orphan files that are not attributed to any crate', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 10, 90),
          fileEntry('/repo/archive/apps/identity-worker/src/lib.rs', 100, 10, 90),
        ],
      },
    ],
  };
  const result = evaluateCoverage(report, examplePackages(), exampleConfig());
  assert.equal(result.orphans.length, 1);
  assert.ok(result.orphans[0].includes('identity-worker'));
});

test('ignores test-support files matching the ignore regex', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 10, 90),
          fileEntry('/repo/libs/rust/email/src/identity.app.tests.rs', 5, 1, 3),
        ],
      },
    ],
  };
  const result = evaluateCoverage(report, examplePackages(), exampleConfig());
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-email').lines, 90);
});
