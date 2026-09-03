import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isDeliveryScopeAffected,
  isTrustedPush,
  isUsableCommitSha,
  selectTypeScriptProjects,
  trustedCacheDirectory,
} from './nx-cache-manager.core.mjs';

const nxConfiguration = JSON.parse(readFileSync('nx.json', 'utf8'));

test('keeps Nx local and disconnected from metered cloud services', () => {
  assert.equal(nxConfiguration.neverConnectToCloud, true);
  assert.equal(nxConfiguration.cacheDirectory, '.nx/cache');
  assert.equal(Object.hasOwn(nxConfiguration, 'nxCloudId'), false);
});

test('only protected branch pushes share the trusted cache', () => {
  for (const ref of ['refs/heads/main', 'refs/heads/dev', 'refs/heads/release/1.0']) {
    assert.equal(isTrustedPush('push', ref), true);
  }
  assert.equal(isTrustedPush('pull_request', 'refs/heads/main'), false);
  assert.equal(isTrustedPush('push', 'refs/heads/staging'), false);
  assert.equal(isTrustedPush('push', 'refs/heads/feature/cache'), false);
});

test('rejects missing and all-zero GitHub base revisions', () => {
  assert.equal(isUsableCommitSha('a'.repeat(40)), true);
  assert.equal(isUsableCommitSha('0'.repeat(40)), false);
  assert.equal(isUsableCommitSha('main'), false);
});

test('versions the persistent cache by platform and Nx major', () => {
  assert.equal(
    trustedCacheDirectory({
      runnerToolCache: '/runner/tools',
      runnerOs: 'macOS',
      runnerArch: 'ARM64',
      nxVersion: '^23.1.1',
    }),
    '/runner/tools/nvbes/nx-cache/v1/macos-arm64/nx-23/trusted',
  );
});

test('selects affected TypeScript projects from the Nx graph', () => {
  const qualityTargets = {
    'format:check': {},
    lint: {},
    typecheck: {},
  };
  const graph = {
    core: { data: { root: 'libs/rust/core' } },
    'identity-sdk-core': {
      data: { root: 'libs/ts/identity-sdk-core', targets: { check: {} } },
    },
    'identity-client': {
      data: { root: 'libs/ts/identity-client', targets: qualityTargets },
    },
    'web-ui': { data: { root: 'libs/ts/web-ui', targets: qualityTargets } },
  };
  assert.deepEqual(
    selectTypeScriptProjects(['core', 'identity-sdk-core', 'web-ui', 'identity-client'], graph),
    ['web-ui', 'identity-client'],
  );
});

test('selects delivery scopes from projects, owned paths, and global inputs', () => {
  const scope = {
    projects: ['account-service', 'account-client'],
    paths: ['apps/account-service', 'infrastructure/environments/account-production'],
  };
  assert.equal(isDeliveryScopeAffected(['account-client'], [], scope), true);
  assert.equal(isDeliveryScopeAffected([], ['apps/account-service/src/main.rs'], scope), true);
  assert.equal(isDeliveryScopeAffected([], ['Cargo.lock'], scope), false);
  assert.equal(isDeliveryScopeAffected([], ['tools/ci/nx-cache-manager.mjs'], scope), true);
  assert.equal(isDeliveryScopeAffected([], ['docs/README.md'], scope), false);
});
