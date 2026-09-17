import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import test from 'node:test';
const require = createRequire(import.meta.url);
// Exercise Nx's installed parser AND reverse graph traversal, not a substitute
// pnpm parser. Nx upgrades must keep this integration contract passing.
const { filterAffected } = require('nx/src/project-graph/affected/affected-project-graph.js');
const { LockFileChange, WholeFileChange } = require('nx/src/project-graph/file-utils.js');
const { output } = require('nx/src/utils/output.js');
const config = { pluginsConfig: { '@nx/js': { projectsAffectedByDependencyUpdates: 'auto' } } };
const nodes = Object.fromEntries(
  ['consumer', 'dependent', 'unrelated'].map((name) => [
    name,
    { name, type: 'lib', data: { root: `libs/${name}`, targets: {} } },
  ]),
);
const graph = {
  nodes,
  externalNodes: {
    'npm:is-odd': {
      name: 'npm:is-odd',
      type: 'npm',
      data: { packageName: 'is-odd', version: '3.0.1' },
    },
    'npm:is-number': {
      name: 'npm:is-number',
      type: 'npm',
      data: { packageName: 'is-number', version: '7.0.0' },
    },
  },
  dependencies: {
    consumer: [{ source: 'consumer', target: 'npm:is-number', type: 'static' }],
    dependent: [{ source: 'dependent', target: 'consumer', type: 'static' }],
    unrelated: [{ source: 'unrelated', target: 'npm:is-odd', type: 'static' }],
    'npm:is-odd': [],
    'npm:is-number': [],
  },
};
const lock = (integrity, other = 'sha512-stable') =>
  `lockfileVersion: '9.0'\nimporters:\n  .: {}\npackages:\n  is-number@7.0.0:\n    resolution: {integrity: ${integrity}}\n  is-odd@3.0.1:\n    resolution: {integrity: ${other}}\nsnapshots:\n  is-number@7.0.0: {}\n  is-odd@3.0.1: {}\n`;
async function affected(change) {
  const result = await filterAffected(
    graph,
    [{ file: 'pnpm-lock.yaml', ext: '.yaml', hash: 'fixture', getChanges: () => [change] }],
    config,
    {},
  );
  return Object.keys(result.nodes).sort();
}
test('Nx lockfile parser selects only consumer and reverse dependents', async () => {
  assert.deepEqual(
    await affected(new LockFileChange(lock('sha512-before'), lock('sha512-after'))),
    ['consumer', 'dependent'],
  );
});
test('unchanged pnpm packages do not affect unrelated libraries', async () => {
  assert.deepEqual(
    await affected(new LockFileChange(lock('sha512-before'), lock('sha512-before'))),
    [],
  );
});
test('grouped npm update selects both consumers and their dependents', async () => {
  assert.deepEqual(
    await affected(new LockFileChange(lock('sha512-before'), lock('sha512-after', 'sha512-other'))),
    ['consumer', 'dependent', 'unrelated'],
  );
});
test('unreadable or malformed lockfile selects all projects', async () => {
  assert.deepEqual(await affected(new WholeFileChange()), ['consumer', 'dependent', 'unrelated']);
  const originalWarn = output.warn.bind(output);
  const warnings = [];
  output.warn = (warning) => {
    warnings.push(warning);
  };
  try {
    assert.deepEqual(await affected(new LockFileChange(lock('sha512-before'), 'invalid: [')), [
      'consumer',
      'dependent',
      'unrelated',
    ]);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0].title, /Failed to parse "pnpm-lock.yaml"/);
  } finally {
    output.warn = originalWarn;
  }
});
