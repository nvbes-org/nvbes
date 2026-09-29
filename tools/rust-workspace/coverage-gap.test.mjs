import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildGapReport, formatGapReport, missingToTarget } from './coverage-gap.mjs';

function examplePackages() {
  return [
    { name: 'nvbes-core', manifest_path: '/repo/libs/rust/core/Cargo.toml' },
    { name: 'nvbes-email', manifest_path: '/repo/libs/rust/email/Cargo.toml' },
  ];
}

function fileEntry(filename, count, covered) {
  return {
    filename,
    summary: {
      lines: { count, covered },
      functions: { count, covered },
      regions: { count, covered },
    },
  };
}

test('missingToTarget returns the covered lines still required', () => {
  assert.equal(missingToTarget([100, 40], 90), 50);
  assert.equal(missingToTarget([100, 95], 90), 0);
  assert.equal(missingToTarget([0, 0], 90), 0);
});

test('ranks crates by the effort remaining to reach the target', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 400, 100),
          fileEntry('/repo/libs/rust/email/src/render.rs', 100, 95),
        ],
      },
    ],
  };
  const gap = buildGapReport(report, examplePackages(), 90);
  assert.equal(gap.crates[0].crate, 'nvbes-core');
  assert.equal(gap.crates[0].missing, 260);
  assert.equal(gap.crates[1].missing, 0);
  assert.equal(gap.workspace.instrumented, 500);
});

test('lists uncovered files per crate, largest gap first', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 100, 90),
          fileEntry('/repo/libs/rust/core/src/config.rs', 100, 10),
          fileEntry('/repo/libs/rust/core/src/done.rs', 50, 50),
        ],
      },
    ],
  };
  const gap = buildGapReport(report, examplePackages(), 90);
  const [core] = gap.crates;
  assert.deepEqual(
    core.files.map((file) => file.path),
    ['core/src/config.rs', 'core/src/limiter.rs'],
  );
});

test('excludes test-support files from the gap ranking', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 100, 90),
          fileEntry('/repo/libs/rust/core/src/limiter.tests.rs', 400, 0),
        ],
      },
    ],
  };
  const gap = buildGapReport(report, examplePackages(), 90);
  assert.equal(gap.workspace.instrumented, 100);
});

test('leaves excluded crates out of the workspace arithmetic', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 100, 50),
          fileEntry('/repo/libs/rust/email/src/render.rs', 100, 0),
        ],
      },
    ],
  };
  const gap = buildGapReport(report, examplePackages(), 90, ['nvbes-email']);
  assert.equal(gap.workspace.instrumented, 100);
  assert.deepEqual(
    gap.crates.map((crate) => crate.crate),
    ['nvbes-core'],
  );
});

test('formats a readable summary capped to the requested file count', () => {
  const report = {
    data: [
      {
        files: [
          fileEntry('/repo/libs/rust/core/src/limiter.rs', 100, 10),
          fileEntry('/repo/libs/rust/core/src/config.rs', 100, 20),
        ],
      },
    ],
  };
  const text = formatGapReport(buildGapReport(report, examplePackages(), 90), 1);
  assert.match(text, /workspace: 15\.0% lines/u);
  assert.match(text, /core\/src\/limiter\.rs/u);
  assert.doesNotMatch(text, /core\/src\/config\.rs/u);
});
