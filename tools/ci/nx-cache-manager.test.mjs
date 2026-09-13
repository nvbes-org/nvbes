import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isTrustedPush,
  isUsableCommitSha,
  isRustWorkspaceAffected,
} from './nx-cache-manager.core.mjs';

test('only protected pushes may publish trusted cache content', () => {
  for (const ref of ['refs/heads/main']) {
    assert.equal(isTrustedPush('push', ref), true);
    assert.equal(isTrustedPush('pull_request', ref), false);
    assert.equal(isTrustedPush('workflow_dispatch', ref), false);
  }
  assert.equal(isTrustedPush('push', 'refs/heads/feature'), false);
  assert.equal(isTrustedPush('push', 'refs/heads/dev'), false);
  assert.equal(isTrustedPush('push', 'refs/heads/release/v1'), false);
});
test('rejects missing and all-zero base revisions', () => {
  for (const sha of ['', '0'.repeat(40), 'main', 'HEAD^', '--help'])
    assert.equal(isUsableCommitSha(sha), false);
  assert.equal(isUsableCommitSha('a'.repeat(40)), true);
});
test('Rust includes compile-time templates but excludes Docker and migrations', () => {
  assert.equal(
    isRustWorkspaceAffected(['libs/rust/email/templates/email.action.generated.html']),
    true,
  );
  assert.equal(isRustWorkspaceAffected(['apps/email-worker/Dockerfile']), false);
  assert.equal(isRustWorkspaceAffected(['apps/email-worker/migrations/002.sql']), false);
});

test('micro-test harness, inventories, and isolated TypeScript select the Rust lane', () => {
  for (const path of [
    'scripts/test-unit.sh',
    'tools/rust-workspace/network-deny.c',
    'tools/rust-workspace/micro-test.components.json',
    'docs/testing/v1/manifest.json',
    'libs/ts/email-ui/src/email-ui.test.tsx',
    'libs/ts/identity-sdk-core/openapi.contract.test.mjs',
    'apps/billing-service/Cargo.lock',
  ])
    assert.equal(isRustWorkspaceAffected([path]), true, path);
});
