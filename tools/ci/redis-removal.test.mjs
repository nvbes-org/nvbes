import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import test from 'node:test';
import { parse } from 'yaml';

test('active Cargo workspaces contain no Redis package or adapter', () => {
  for (const file of ['Cargo.lock', 'apps/billing-service/Cargo.lock'].filter(existsSync)) {
    assert.doesNotMatch(readFileSync(file, 'utf8'), /^name = "(?:redis|nvbes-redis|bb8-redis)"$/mu);
  }
  assert.equal(existsSync('libs/rust/redis/Cargo.toml'), false);
});

test('local service definitions do not provision Redis', () => {
  for (const file of [
    '.devcontainer/docker-compose.yml',
    'infrastructure/local/docker-compose.yml',
    'infrastructure/local/identity-e2e.compose.yml',
  ]) {
    const compose = parse(readFileSync(file, 'utf8'));
    assert.equal(compose.services.redis, undefined);
    for (const service of Object.values(compose.services)) {
      assert.doesNotMatch(JSON.stringify(service), /redis/iu);
    }
  }
});

test('Rust CI provisions and always cleans up security PostgreSQL', () => {
  const { jobs } = parse(readFileSync('.github/workflows/ci.yml', 'utf8'));
  const steps = jobs.rust.steps;
  const setup = steps.findIndex((s) => s.run === 'node tools/ci/test-security-database.mjs');
  const run = steps.findIndex((s) => s.run === 'node tools/ci/run-rust.mjs');
  const cleanup = steps.find((s) => s.run === 'node tools/ci/test-security-database.mjs stop');
  assert(setup >= 0 && setup < run);
  assert.equal(cleanup?.if, 'always()');
  assert.doesNotMatch(JSON.stringify(jobs), /redis/iu);
});
