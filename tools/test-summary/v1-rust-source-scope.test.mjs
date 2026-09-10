import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { rustSources } from './v1-catalogue.mjs';
import { rustCoverage } from './v1-rust-measurements.mjs';

test('Cargo inventory includes root build scripts and nested source modules', (t) => {
  const cwd = mkdtempSync(path.join(os.tmpdir(), 'nvbes-rust-inventory-'));
  t.after(() => rmSync(cwd, { recursive: true, force: true }));
  const directory = path.join(cwd, 'example');
  mkdirSync(path.join(directory, 'src/nested'), { recursive: true });
  for (const file of ['build.rs', 'src/lib.rs', 'src/nested/module.rs', 'src/lib.tests.rs'])
    writeFileSync(path.join(directory, file), '');
  const inventory = rustSources(path.join(directory, 'Cargo.toml'), cwd);
  assert.equal(inventory.sourceRoot, 'example');
  assert.deepEqual(inventory.sources.sort(), [
    'example/build.rs',
    'example/src/lib.rs',
    'example/src/lib.tests.rs',
    'example/src/nested/module.rs',
  ]);
  assert.throws(() => rustSources(path.join(cwd, 'Cargo.toml'), directory), /outside/u);
});

test('LLVM scoring isolates each crate and excludes explicit test source files', () => {
  const unit = {
    name: 'example',
    sourceRoot: 'libs/rust/example',
    sources: ['libs/rust/example/src/lib.rs', 'libs/rust/example/src/lib.tests.rs'],
  };
  const entry = (filename, count, covered) => ({
    filename,
    summary: { lines: { count, covered } },
  });
  const report = {
    type: 'llvm.coverage.json.export',
    version: '3.0.1',
    data: [
      {
        files: [
          entry(unit.sources[0], 100, 20),
          entry(unit.sources[1], 10000, 10000),
          entry('libs/rust/example-other/src/lib.rs', 10000, 10000),
        ],
      },
    ],
  };
  assert.equal(rustCoverage(unit, report, 'lines'), 20);
  report.data[0].files.push(entry(`/tmp/${unit.sources[0]}`, 100, 100));
  assert.throws(() => rustCoverage(unit, report, 'lines'), /Duplicate/u);
});
