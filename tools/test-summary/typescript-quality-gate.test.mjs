import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { enforceTypescriptThresholds, typescriptScope } from './typescript-quality-gate.mjs';
import { typescriptSources } from './v1-catalogue.mjs';
import { typescriptCoverage } from './v1-typescript-measurements.mjs';

const unit = { name: '@nvbes/example', thresholds: {} };
const thresholds = { lines: 90, branches: 90, mutation: 90 };

for (const metric of Object.keys(thresholds)) {
  test(`${metric} rejects 89.99, nonfinite values and weaker thresholds`, () => {
    for (const score of [89.99, Number.NaN, Infinity, -Infinity]) {
      assert.throws(() => enforceTypescriptThresholds(unit, thresholds, { [metric]: score }));
    }
    assert.throws(() =>
      enforceTypescriptThresholds(unit, { ...thresholds, [metric]: 1 }, { [metric]: 89.99 }),
    );
    assert.throws(() =>
      enforceTypescriptThresholds({ ...unit, thresholds: { [metric]: 95 } }, thresholds, {
        [metric]: 94.99,
      }),
    );
    assert.deepEqual(enforceTypescriptThresholds(unit, thresholds, { [metric]: 90 }), {
      [metric]: 90,
    });
    for (const value of [undefined, null, '90', Number.NaN, 101, -1]) {
      assert.throws(() =>
        enforceTypescriptThresholds(unit, { ...thresholds, [metric]: value }, { [metric]: 100 }),
      );
    }
  });
}

test('only pure export wiring can have no instrumented lines', (t) => {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-barrel-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  mkdirSync(path.join(directory, 'src'));
  const sources = {
    'index.ts': "export { enabled } from './value';",
    'value.ts': 'export const enabled = true;',
    'effects.ts': "import './value'; export {};",
  };
  for (const [name, source] of Object.entries(sources))
    writeFileSync(path.join(directory, 'src', name), source);
  const inventory = typescriptSources(directory, 'libs/ts/example');
  assert.equal(inventory['libs/ts/example/src/index.ts'].forwardingOnly, true);
  assert.equal(inventory['libs/ts/example/src/value.ts'].forwardingOnly, undefined);
  assert.equal(inventory['libs/ts/example/src/effects.ts'].forwardingOnly, undefined);
  const report = Object.fromEntries(
    Object.keys(inventory).map((file) => [
      file,
      {
        lines: {
          total: file.endsWith('index.ts') ? 0 : 1,
          covered: file.endsWith('index.ts') ? 0 : 1,
          skipped: 0,
        },
        branches: { total: 1, covered: 1, skipped: 0 },
      },
    ]),
  );
  const candidate = { ...unit, sourceFiles: inventory };
  assert.deepEqual(typescriptCoverage(candidate, report), { lines: 100, branches: 100 });
  delete report['libs/ts/example/src/index.ts'];
  assert.deepEqual(typescriptCoverage(candidate, report), { lines: 100, branches: 100 });
  report['libs/ts/example/src/effects.ts'].lines = { total: 0, covered: 0, skipped: 0 };
  assert.throws(() => typescriptCoverage(candidate, report), /Runtime source has no lines/u);
});

test('local package selection refuses archived, generated, unknown and unsafe names', () => {
  for (const name of [
    'identity-sdk-core',
    'backoffice-service-sdk-core',
    'unknown',
    '../email-ui',
    'email-ui;echo',
  ]) {
    assert.throws(() => typescriptScope(name));
  }
  assert.equal(typescriptScope('email-ui').unit.name, '@nvbes/email-ui');
});
