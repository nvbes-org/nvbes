import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const checker = fileURLToPath(new URL('./check-scale-to-zero.mjs', import.meta.url));

async function runChecker(terraform, t) {
  const directory = await mkdtemp(path.join(tmpdir(), 'nvbes-scale-to-zero-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(path.join(directory, 'runtime.tf'), terraform);
  return spawnSync(process.execPath, [checker, directory], { encoding: 'utf8' });
}

test('accepts bounded zero-minimum Scaleway runtimes', async (t) => {
  const result = await runChecker(`
resource "scaleway_container" "api" {
  min_scale = 0
  max_scale = 1
}
resource "scaleway_sdb_sql_database" "api" {
  min_cpu = 0
  max_cpu = 1
}
`, t);

  assert.equal(result.status, 0, result.stderr);
});

test('rejects unbounded or non-scale-to-zero Scaleway runtimes', async (t) => {
  const result = await runChecker(`
resource "scaleway_container" "api" {
  min_scale = 1
  max_scale = 1
}
resource "scaleway_container" "worker" {
  min_scale = 0
  max_scale = 10
}
resource "scaleway_sdb_sql_database" "api" {
  min_cpu = 0
}
resource "scaleway_sdb_sql_database" "worker" {
  min_cpu = 0
  max_cpu = 2
}
`, t);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /scaleway_container must declare min_scale = 0/);
  assert.match(result.stderr, /scaleway_container must declare max_scale as an integer <= 1/);
  assert.match(result.stderr, /scaleway_sdb_sql_database must declare max_cpu as an integer <= 1/);
});
