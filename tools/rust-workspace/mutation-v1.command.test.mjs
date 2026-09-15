import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { mutationCommand } from './mutation-v1.command.mjs';

test('mutation resolves root and nested manifests to existing parent paths', () => {
  for (const manifest of ['Cargo.toml', 'apps/billing-service/Cargo.toml']) {
    const args = mutationCommand(manifest, ['nvbes-core', 'nvbes-billing'], '/tmp/output', {});
    assert.equal(args[2], path.resolve(manifest));
    assert(args.includes('--cargo-arg=--locked'));
    assert(!args.includes('--baseline=skip'));
    assert.deepEqual(args.slice(3, 7), ['--package', 'nvbes-core', '--package', 'nvbes-billing']);
  }
});
test('mutation rejects unlimited concurrency and missing timeouts', () => {
  for (const env of [
    { NVBES_MUTATION_JOBS: '0' },
    { NVBES_MUTATION_JOBS: '5' },
    { NVBES_MUTATION_TIMEOUT: '0' },
    { NVBES_MUTATION_TIMEOUT: '' },
  ]) {
    assert.throws(() => mutationCommand('Cargo.toml', ['nvbes-core'], '/tmp/output', env));
  }
});
