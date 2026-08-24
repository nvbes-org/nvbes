import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const checker = fileURLToPath(new URL('./check-scale-to-zero.mjs', import.meta.url));

async function runChecker(terraform) {
  const directory = await mkdtemp(path.join(tmpdir(), 'nvbes-scale-to-zero-'));
  await writeFile(path.join(directory, 'runtime.tf'), terraform);
  return spawnSync(process.execPath, [checker, directory], { encoding: 'utf8' });
}

test('accepts zero-minimum Scaleway runtimes', async () => {
  const result = await runChecker(`
resource "scaleway_container" "api" {
  min_scale = 0
}
resource "scaleway_sdb_sql_database" "api" {
  min_cpu = 0
}
`);

  assert.equal(result.status, 0, result.stderr);
});

test('rejects an always-on or unspecified minimum', async () => {
  const result = await runChecker(`
resource "scaleway_container" "api" {
  min_scale = 1
}
resource "scaleway_sdb_sql_database" "api" {
  max_cpu = 2
}
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /scaleway_container must declare min_scale = 0/);
  assert.match(result.stderr, /scaleway_sdb_sql_database must declare min_cpu = 0/);
});
