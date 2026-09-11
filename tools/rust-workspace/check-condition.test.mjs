import assert from 'node:assert/strict';
import { test } from 'node:test';
import { evaluateCondition, formatConditionTable } from './check-condition.mjs';

function examplePackages() {
  return [
    { name: 'nvbes-email', manifest_path: '/repo/libs/rust/email/Cargo.toml' },
    { name: 'nvbes-dpop', manifest_path: '/repo/libs/rust/dpop/Cargo.toml' },
    {
      name: 'nvbes-trust-risk',
      manifest_path: '/repo/libs/rust/trust-risk/Cargo.toml',
    },
    { name: 'nvbes-scan', manifest_path: '/repo/libs/rust/scan/Cargo.toml' },
    { name: 'nvbes-ports', manifest_path: '/repo/libs/rust/ports/Cargo.toml' },
  ];
}

function fileEntry(filename, branchesCount, branchesCovered) {
  return {
    filename,
    summary: {
      lines: { count: 0, covered: 0 },
      functions: { count: 0, covered: 0 },
      regions: { count: 0, covered: 0 },
      branches: { count: branchesCount, covered: branchesCovered },
    },
  };
}

function exampleConfig() {
  return {
    schemaVersion: 1,
    inclusionFloor: 50,
    crates: {
      'nvbes-email': { threshold: 87, baseline: { score: 91.7 } },
      'nvbes-dpop': { threshold: 60, baseline: { score: 65.4 } },
      'nvbes-trust-risk': { threshold: 53, baseline: { score: 58.3 } },
    },
    excluded: [
      {
        crate: 'nvbes-scan',
        reason: 'skeleton without tests',
        baseline: { score: 0.0 },
      },
      {
        crate: 'nvbes-ports',
        reason: 'port contracts only',
        baseline: { score: 0.0 },
      },
    ],
  };
}

test('scores branch coverage per crate from the JSON report and passes', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 100),
          fileEntry('/repo/libs/rust/dpop/src/lib.rs', 20, 15),
          fileEntry('/repo/libs/rust/trust-risk/src/identity.app.rs', 50, 30),
        ],
      },
    ],
  };
  const result = evaluateCondition(report, examplePackages(), exampleConfig());
  assert.deepEqual(result.failures, []);
  assert.equal(result.rows.length, 3);
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-email').score, 100);
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-dpop').score, 75);
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-trust-risk').score, 60);
});

test('detects a condition coverage gap below its threshold', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 80),
          fileEntry('/repo/libs/rust/dpop/src/lib.rs', 20, 10),
          fileEntry('/repo/libs/rust/trust-risk/src/identity.app.rs', 50, 30),
        ],
      },
    ],
  };
  const result = evaluateCondition(report, examplePackages(), exampleConfig());
  const email = result.rows.find((row) => row.crate === 'nvbes-email');
  assert.ok(email.gap);
  assert.equal(email.score, 80);
  assert.ok(result.failures.some((failure) => failure.startsWith('nvbes-email')));
});

test('flags a configured crate absent from the report as a gap', () => {
  const report = { data: [{ files: [] }] };
  const result = evaluateCondition(report, examplePackages(), exampleConfig());
  assert.ok(result.rows.find((row) => row.crate === 'nvbes-email').gap);
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-email').score, 0);
});

test('rejects an unsupported schema version', () => {
  const config = { schemaVersion: 2, crates: {}, excluded: [] };
  assert.throws(
    () => evaluateCondition({ data: [] }, examplePackages(), config),
    /Unsupported condition schemaVersion/u,
  );
});

test('rejects a crate that is both configured and excluded', () => {
  const config = {
    schemaVersion: 1,
    crates: { 'nvbes-email': { threshold: 87, baseline: { score: 91.7 } } },
    excluded: [{ crate: 'nvbes-email', reason: 'whoops', baseline: { score: 91.7 } }],
  };
  assert.throws(
    () => evaluateCondition({ data: [] }, examplePackages(), config),
    /both configured and excluded/u,
  );
});

test('rejects a configured crate that is not a workspace member', () => {
  const config = {
    schemaVersion: 1,
    crates: { 'nvbes-ghost': { threshold: 0, baseline: { score: null } } },
    excluded: [],
  };
  assert.throws(
    () => evaluateCondition({ data: [] }, examplePackages(), config),
    /not a workspace member/u,
  );
});

test('rejects an excluded crate without a justification or baseline', () => {
  const config = {
    schemaVersion: 1,
    crates: {},
    excluded: [{ crate: 'nvbes-scan', reason: '', baseline: { score: null } }],
  };
  assert.throws(
    () => evaluateCondition({ data: [] }, examplePackages(), config),
    /explicit reason/u,
  );
});

test('ignores test-support files matching the ignore regex', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/email/src/identity.app.rs', 100, 80),
          fileEntry('/repo/libs/rust/email/src/identity.app.tests.rs', 100, 100),
        ],
      },
    ],
  };
  const result = evaluateCondition(report, examplePackages(), exampleConfig());
  const email = result.rows.find((row) => row.crate === 'nvbes-email');
  assert.equal(email.count, 100);
  assert.equal(email.covered, 80);
});

test('formats the report table for display', () => {
  const result = evaluateCondition(
    {
      data: [
        {
          files: [
            fileEntry('/repo/libs/rust/email/src/identity.app.rs', 2, 2),
            fileEntry('/repo/libs/rust/dpop/src/lib.rs', 2, 1),
            fileEntry('/repo/libs/rust/trust-risk/src/identity.app.rs', 2, 1),
          ],
        },
      ],
    },
    examplePackages(),
    exampleConfig(),
  );
  const table = formatConditionTable(result.rows);
  assert.ok(table.includes('nvbes-email'));
  assert.ok(table.includes('branches'));
  assert.match(table, /100\.0%/u);
});
