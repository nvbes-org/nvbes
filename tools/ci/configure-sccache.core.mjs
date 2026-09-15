import { isTrustedPush } from './nx-cache-manager.core.mjs';

export function compilationCacheEnvironment(source) {
  const env = { ...source };
  // Never inherit a backend from a previous job on a self-hosted runner.
  for (const key of Object.keys(env)) {
    if (
      key.startsWith('SCCACHE_S3_') ||
      key.startsWith('SCCACHE_GHA_') ||
      ['SCCACHE_BUCKET', 'SCCACHE_ENDPOINT', 'SCCACHE_REGION'].includes(key)
    )
      delete env[key];
  }
  const pullRequest = env.GITHUB_EVENT_NAME === 'pull_request';
  const githubCache =
    env.GITHUB_ACTIONS === 'true' &&
    pullRequest &&
    /^refs\/pull\/\d+\/merge$/u.test(env.GITHUB_REF ?? '') &&
    env.ACTIONS_RESULTS_URL &&
    env.ACTIONS_RUNTIME_TOKEN;
  const configured = env.AWS_ACCESS_KEY_ID && env.AWS_SECRET_ACCESS_KEY && env.SCW_CI_CACHE_BUCKET;
  if (githubCache) {
    // GitHub restricts writes to this PR merge ref; main cannot restore them.
    env.SCCACHE_GHA_ENABLED = 'on';
    env.SCCACHE_GHA_VERSION = `nvbes-rust-v1-${env.RUNNER_OS}-${env.RUNNER_ARCH}`;
    env.SCCACHE_GHA_RW_MODE = 'READ_WRITE';
  } else if (configured && !pullRequest) {
    env.SCCACHE_BUCKET = env.SCW_CI_CACHE_BUCKET;
    env.SCCACHE_ENDPOINT = env.SCW_CI_CACHE_S3_ENDPOINT;
    env.SCCACHE_REGION = env.SCW_CI_CACHE_REGION;
    env.SCCACHE_S3_USE_SSL = 'true';
    env.SCCACHE_S3_ENABLE_VIRTUAL_HOST_STYLE = 'true';
    env.SCCACHE_S3_KEY_PREFIX = `trusted/rust/${env.RUNNER_OS}-${env.RUNNER_ARCH}/rust-1.98.1`;
    env.SCCACHE_S3_RW_MODE = isTrustedPush(env.GITHUB_EVENT_NAME, env.GITHUB_REF)
      ? 'READ_WRITE'
      : 'READ_ONLY';
  }
  if (!env.SCCACHE_BUCKET) {
    delete env.AWS_ACCESS_KEY_ID;
    delete env.AWS_SECRET_ACCESS_KEY;
    delete env.AWS_SESSION_TOKEN;
  }
  return env;
}
