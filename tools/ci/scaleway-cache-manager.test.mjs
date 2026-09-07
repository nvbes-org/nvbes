import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { isTrustedRef, restorePrefixes, writablePrefix } from './scaleway-cache-manager.core.mjs';

const manager = readFileSync(
  new URL('./scaleway-cache-manager.mjs', import.meta.url),
  'utf8',
).replaceAll("'", '"');

test('the manager accepts the shared sccache S3 environment', () => {
  assert.match(manager, /process\.env\.SCCACHE_BUCKET/u);
  assert.match(manager, /process\.env\.SCCACHE_ENDPOINT/u);
  assert.match(manager, /process\.env\.SCCACHE_REGION/u);
});

test('dependency archives are immutable and Nx is synchronized incrementally', () => {
  assert.match(manager, /fileDigest\(\["pnpm-lock\.yaml"\]\)/u);
  assert.match(manager, /terraformLockfiles\(\)/u);
  assert.match(manager, /"s3api",\n\s+"head-object"/u);
  assert.match(manager, /definition\.mode === "sync"/u);
  assert.match(manager, /"s3",\n\s+"sync"/u);
  // SQLite can change without changing its file size; publication must compare timestamps.
  assert.doesNotMatch(manager, /"--size-only"/u);
});

test('protected refs share the trusted namespace', () => {
  for (const ref of ['refs/heads/main']) {
    assert.equal(isTrustedRef(ref), true);
    assert.deepEqual(restorePrefixes({ ref }), ['trusted']);
    assert.equal(writablePrefix({ eventName: 'push', ref }), 'trusted');
  }
});

test('branches restore only trusted archives and never publish', () => {
  for (const ref of ['refs/heads/dev', 'refs/heads/release/v1']) {
    assert.equal(isTrustedRef(ref), false);
    assert.equal(writablePrefix({ eventName: 'push', ref }), null);
  }
  const environment = {
    eventName: 'push',
    ref: 'refs/heads/feature/cache',
    refName: 'feature/cache',
  };
  assert.deepEqual(restorePrefixes(environment), ['trusted']);
  assert.equal(writablePrefix(environment), null);
});

test('pull requests never publish archives or restore untrusted archives', () => {
  const environment = {
    eventName: 'pull_request',
    ref: 'refs/pull/42/merge',
    headRef: 'feature/cache',
  };
  assert.equal(writablePrefix(environment), null);
  assert.deepEqual(restorePrefixes(environment), ['trusted']);
});
