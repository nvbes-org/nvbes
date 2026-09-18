import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { sha256 } from './v1-bundle-verification.mjs';
import { typescriptSources } from './v1-catalogue.mjs';
import {
  typescriptMeasurements,
  verifyTypescriptMeasurements,
} from './v1-typescript-measurements.mjs';

const file = 'libs/ts/example/src/index.ts';
const source = 'export const enabled = true;';
function fixture() {
  const unit = {
    name: '@nvbes/example',
    language: 'typescript',
    sourceFiles: {
      [file]: { sha256: sha256(source), runtime: true },
    },
  };
  const coverage = {
    [file]: {
      lines: { total: 10000, covered: 8999, skipped: 0, pct: 100 },
      branches: { total: 3, covered: 2, skipped: 0, pct: 100 },
    },
    total: { lines: { total: 1, covered: 1, pct: 100 } },
  };
  const mutation = {
    schemaVersion: '1.0',
    files: {
      [file]: {
        source,
        mutants: [
          { id: '0', status: 'Killed' },
          { id: '1', status: 'Timeout' },
        ],
      },
    },
  };
  return { unit, coverage, mutation };
}

test('uses per-file integer counters, ignores advertised pct and totals, and counts timeout as missed', () => {
  const { unit, coverage, mutation } = fixture();
  assert.deepEqual(typescriptMeasurements(unit, coverage, mutation), {
    lines: 89.99,
    branches: 200 / 3,
    mutation: 50,
  });
});

test('accepts CI absolute paths while requiring every runtime source', () => {
  const { unit, coverage, mutation } = fixture();
  coverage[`/home/runner/work/nvbes/nvbes/${file}`] = coverage[file];
  delete coverage[file];
  assert.equal(typescriptMeasurements(unit, coverage, mutation).lines, 89.99);
});

test('declarative sources may be absent or empty, but cannot inflate mutation scores', () => {
  const { unit, coverage, mutation } = fixture();
  const types = 'libs/ts/example/src/types.ts';
  const declaration = 'export interface Input { value: string }';
  unit.sourceFiles[types] = { runtime: false, sha256: sha256(declaration) };
  assert.equal(typescriptMeasurements(unit, coverage, mutation).mutation, 50);
  coverage[types] = {
    lines: { total: 0, covered: 0, skipped: 0 },
    branches: { total: 0, covered: 0, skipped: 0 },
  };
  mutation.files[types] = { source: declaration, mutants: [] };
  assert.equal(typescriptMeasurements(unit, coverage, mutation).mutation, 50);
  mutation.files[types].mutants.push({ id: '2', status: 'Killed' });
  assert.throws(
    () => typescriptMeasurements(unit, coverage, mutation),
    /Declarative source has mutants/u,
  );
});

for (const [name, change] of [
  [
    'omitted coverage source',
    (f) => {
      delete f.coverage[file];
    },
  ],
  [
    'omitted mutation source',
    (f) => {
      delete f.mutation.files[file];
    },
  ],
  [
    'different candidate source',
    (f) => {
      f.mutation.files[file].source += '\n';
    },
  ],
  [
    'unknown source',
    (f) => {
      f.coverage['other.ts'] = f.coverage[file];
    },
  ],
  [
    'duplicate source',
    (f) => {
      f.coverage[`/tmp/${file}`] = f.coverage[file];
    },
  ],
  [
    'traversal',
    (f) => {
      f.coverage[`../${file}`] = f.coverage[file];
    },
  ],
  [
    'fake mutation total',
    (f) => {
      f.mutation.files.total = f.mutation.files[file];
    },
  ],
  [
    'invalid schema',
    (f) => {
      f.mutation.schemaVersion = '2.0';
    },
  ],
  [
    'partial mutation result',
    (f) => {
      f.mutation.files[file].mutants[0].status = 'Pending';
    },
  ],
  [
    'duplicate mutant',
    (f) => {
      f.mutation.files[file].mutants[1].id = '0';
    },
  ],
  [
    'ignored mutant',
    (f) => {
      f.mutation.files[file].mutants[0].status = 'Ignored';
    },
  ],
  [
    'skipped coverage',
    (f) => {
      f.coverage[file].lines.skipped = 1;
    },
  ],
  [
    'fractional counts',
    (f) => {
      f.coverage[file].lines.total = 10000.5;
    },
  ],
  [
    'overcount',
    (f) => {
      f.coverage[file].branches.covered = 4;
    },
  ],
  [
    'negative counts',
    (f) => {
      f.coverage[file].branches.covered = -1;
    },
  ],
  [
    'zero runtime lines',
    (f) => {
      f.coverage[file].lines.total = 0;
      f.coverage[file].lines.covered = 0;
    },
  ],
  [
    'zero applicable branches',
    (f) => {
      f.coverage[file].branches.total = 0;
      f.coverage[file].branches.covered = 0;
    },
  ],
  [
    'declarative counters',
    (f) => {
      f.unit.sourceFiles[file].runtime = false;
    },
  ],
]) {
  test(`rejects ${String(name)}`, () => {
    const f = fixture();
    change(f);
    assert.throws(() => typescriptMeasurements(f.unit, f.coverage, f.mutation));
  });
}

test('requires authenticated report references and exact unrounded declared measurements', () => {
  const { unit, coverage, mutation } = fixture();
  const artifacts = new Map([
    ['coverage.json', Buffer.from(JSON.stringify(coverage))],
    ['mutation.json', Buffer.from(JSON.stringify(mutation))],
  ]);
  const measurement = {
    unit: unit.name,
    ...typescriptMeasurements(unit, coverage, mutation),
    artifacts: [...artifacts.keys()],
    reports: { 'istanbul-summary': 'coverage.json', stryker: 'mutation.json' },
  };
  const bundle = { measurements: [measurement] };
  assert.doesNotThrow(() => verifyTypescriptMeasurements(bundle, [unit], artifacts));
  for (const metric of ['lines', 'branches', 'mutation']) {
    const original = measurement[metric];
    measurement[metric] = 100;
    assert.throws(() => verifyTypescriptMeasurements(bundle, [unit], artifacts), /differs/u);
    measurement[metric] = original;
  }
  assert.throws(() => verifyTypescriptMeasurements({ measurements: [] }, [unit], artifacts));
  assert.throws(() =>
    verifyTypescriptMeasurements({ measurements: [measurement, measurement] }, [unit], artifacts),
  );
  measurement.artifacts = ['coverage.json'];
  assert.throws(() => verifyTypescriptMeasurements(bundle, [unit], artifacts), /verified/u);
  measurement.artifacts = [...artifacts.keys()];
  artifacts.delete('mutation.json');
  assert.throws(() => verifyTypescriptMeasurements(bundle, [unit], artifacts), /verified/u);
});

test('candidate inventory hashes runtime sources and classifies declarative code without excluding runtime', (t) => {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-ts-inventory-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  mkdirSync(path.join(directory, 'src'));
  for (const [name, contents] of Object.entries({
    'index.ts': source,
    'types.ts': 'export interface Input { value: string }',
    'index.test.ts': 'throw new Error();',
    'types.d.ts': 'declare const value: number;',
  }))
    writeFileSync(path.join(directory, 'src', name), contents);
  const result = typescriptSources(directory, 'libs/ts/example');
  assert.equal(Object.keys(result).length, 2);
  assert.deepEqual(result[file], { runtime: true, sha256: sha256(source) });
  assert.equal(result['libs/ts/example/src/types.ts'].runtime, false);
});
