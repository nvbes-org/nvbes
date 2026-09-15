import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse } from 'yaml';
import { compilationCacheEnvironment } from './configure-sccache.core.mjs';

const github = {
  GITHUB_ACTIONS: 'true',
  GITHUB_EVENT_NAME: 'pull_request',
  GITHUB_REF: 'refs/pull/243/merge',
  RUNNER_OS: 'Linux',
  RUNNER_ARCH: 'X64',
  ACTIONS_RESULTS_URL: 'https://cache.invalid',
  ACTIONS_RUNTIME_TOKEN: 'synthetic-runtime-token',
};
const scaleway = {
  AWS_ACCESS_KEY_ID: 'synthetic-key',
  AWS_SECRET_ACCESS_KEY: 'synthetic-secret',
  SCW_CI_CACHE_BUCKET: 'synthetic-bucket',
  SCW_CI_CACHE_S3_ENDPOINT: 'https://s3.invalid',
  SCW_CI_CACHE_REGION: 'fr-par',
};

test('PRs use the native ref-scoped cache without Scaleway credentials', () => {
  for (const extra of [{}, scaleway, { GITHUB_ACTOR: 'dependabot[bot]' }]) {
    const env = compilationCacheEnvironment({ ...github, ...extra });
    assert.equal(env.SCCACHE_GHA_ENABLED, 'on');
    assert.equal(env.SCCACHE_GHA_RW_MODE, 'READ_WRITE');
    assert.equal(env.SCCACHE_BUCKET, undefined);
    assert.equal(env.AWS_SECRET_ACCESS_KEY, undefined);
  }
});
test('main retains the trusted S3 namespace and never restores the PR backend', () => {
  const env = compilationCacheEnvironment({
    ...github,
    ...scaleway,
    GITHUB_EVENT_NAME: 'push',
    GITHUB_REF: 'refs/heads/main',
    SCCACHE_GHA_ENABLED: 'on',
  });
  assert.equal(env.SCCACHE_GHA_ENABLED, undefined);
  assert.equal(env.SCCACHE_BUCKET, scaleway.SCW_CI_CACHE_BUCKET);
  assert.equal(env.SCCACHE_S3_RW_MODE, 'READ_WRITE');
});
test('untrusted refs cannot publish to S3 and incomplete PR credentials use local disk', () => {
  const branch = compilationCacheEnvironment({
    ...github,
    ...scaleway,
    GITHUB_EVENT_NAME: 'push',
    GITHUB_REF: 'refs/heads/topic',
  });
  assert.equal(branch.SCCACHE_S3_RW_MODE, 'READ_ONLY');
  for (const key of ['ACTIONS_RESULTS_URL', 'ACTIONS_RUNTIME_TOKEN']) {
    const env = compilationCacheEnvironment({ ...github, ...scaleway, [key]: '' });
    assert.equal(env.SCCACHE_GHA_ENABLED, undefined);
    assert.equal(env.SCCACHE_BUCKET, undefined);
  }
});
test('local runs and invalid PR refs cannot enable GHA; stale backends are removed', () => {
  for (const extra of [{ GITHUB_ACTIONS: '' }, { GITHUB_REF: 'refs/heads/main' }]) {
    const env = compilationCacheEnvironment({
      ...github,
      ...extra,
      SCCACHE_GHA_ENABLED: 'on',
      SCCACHE_BUCKET: 'stale',
      SCCACHE_S3_RW_MODE: 'READ_WRITE',
    });
    assert.equal(env.SCCACHE_GHA_ENABLED, undefined);
    assert.equal(env.SCCACHE_BUCKET, undefined);
    assert.equal(env.SCCACHE_S3_RW_MODE, undefined);
  }
});
test('cache setup precedes coverage installation and publishes real statistics', () => {
  const steps = parse(readFileSync('.github/actions/ci-setup/action.yml', 'utf8')).runs.steps;
  const action = steps.findIndex((step) =>
    step.uses?.startsWith('mozilla-actions/sccache-action@'),
  );
  const configure = steps.findIndex((step) => step.name === 'Configure compilation cache');
  const install = steps.findIndex((step) => step.name === 'Install cargo-llvm-cov');
  assert(action >= 0 && configure > action && install > configure);
  assert.equal(steps[action].with.disable_annotations, false);
});
