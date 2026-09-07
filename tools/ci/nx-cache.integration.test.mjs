import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

const native = createRequire(import.meta.url).resolve('nx/src/native');
const script = `
  const { NxCache, TaskDetails, connectToNxDb, closeDbConnection } = require(process.env.NX_NATIVE);
  const { mkdirSync } = require('node:fs');
  const { join } = require('node:path');
  const root = process.env.CACHE_FIXTURE;
  const cachePath = join(root, '.nx/cache');
  const dataPath = join(root, process.env.INDEX_PATH);
  mkdirSync(dataPath, { recursive: true });
  const db = connectToNxDb(dataPath);
  const cache = new NxCache(root, cachePath, db);
  const hash = '12345678901234567890';
  if (process.env.CACHE_ACTION === 'put') {
    new TaskDetails(db).recordTaskDetails([{ hash, project: 'fixture', target: 'test' }]);
    cache.put(hash, 'cached task output', [], 0);
  }
  else console.log(JSON.stringify(cache.get(hash)));
  closeDbConnection(db);
`;

for (const [name, indexPath, expectedHit] of [
  ['artifacts without the SQLite index miss', '.nx/workspace-data', false],
  [
    'artifacts and SQLite index survive transfer to a fresh runner',
    '.nx/cache/workspace-data',
    true,
  ],
]) {
  test(name, () => {
    const directory = mkdtempSync(join(tmpdir(), 'nvbes-nx-cache-'));
    const source = join(directory, 'source');
    const destination = join(directory, 'destination');
    const execute = (root, action) =>
      execFileSync(process.execPath, ['-e', script], {
        encoding: 'utf8',
        env: {
          ...process.env,
          NX_NATIVE: native,
          CACHE_FIXTURE: root,
          INDEX_PATH: indexPath,
          CACHE_ACTION: action,
        },
      });
    try {
      execute(source, 'put');
      cpSync(join(source, '.nx/cache'), join(destination, '.nx/cache'), { recursive: true });
      const result = JSON.parse(execute(destination, 'get'));
      assert.equal(result !== null, expectedHit);
      if (expectedHit) {
        assert.equal(result.code, 0);
        assert.equal(result.terminalOutput, 'cached task output');
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}
